#![feature(prelude_import)]
/*!
Generate types for C-style flags with ergonomic APIs.

# Getting started

Add `bitflags` to your `Cargo.toml`:

```toml
[dependencies.bitflags]
version = "2.6.0"
```

## Generating flags types

Use the [`bitflags`] macro to generate flags types:

```rust
use bitflags::bitflags;

bitflags! {
    pub struct Flags: u32 {
        const A = 0b00000001;
        const B = 0b00000010;
        const C = 0b00000100;
    }
}
```

See the docs for the `bitflags` macro for the full syntax.

Also see the [`example_generated`](./example_generated/index.html) module for an example of what the `bitflags` macro generates for a flags type.

### Externally defined flags

If you're generating flags types for an external source, such as a C API, you can define
an extra unnamed flag as a mask of all bits the external source may ever set. Usually this would be all bits (`!0`):

```rust
# use bitflags::bitflags;
bitflags! {
    pub struct Flags: u32 {
        const A = 0b00000001;
        const B = 0b00000010;
        const C = 0b00000100;

        // The source may set any bits
        const _ = !0;
    }
}
```

Why should you do this? Generated methods like `all` and truncating operators like `!` only consider
bits in defined flags. Adding an unnamed flag makes those methods consider additional bits,
without generating additional constants for them. It helps compatibility when the external source
may start setting additional bits at any time. The [known and unknown bits](#known-and-unknown-bits)
section has more details on this behavior.

### Custom derives

You can derive some traits on generated flags types if you enable Cargo features. The following
libraries are currently supported:

- `serde`: Support `#[derive(Serialize, Deserialize)]`, using text for human-readable formats,
and a raw number for binary formats.
- `arbitrary`: Support `#[derive(Arbitrary)]`, only generating flags values with known bits.
- `bytemuck`: Support `#[derive(Pod, Zeroable)]`, for casting between flags values and their
underlying bits values.

You can also define your own flags type outside of the [`bitflags`] macro and then use it to generate methods.
This can be useful if you need a custom `#[derive]` attribute for a library that `bitflags` doesn't
natively support:

```rust
# use std::fmt::Debug as SomeTrait;
# use bitflags::bitflags;
#[derive(SomeTrait)]
pub struct Flags(u32);

bitflags! {
    impl Flags: u32 {
        const A = 0b00000001;
        const B = 0b00000010;
        const C = 0b00000100;
    }
}
```

### Adding custom methods

The [`bitflags`] macro supports attributes on generated flags types within the macro itself, while
`impl` blocks can be added outside of it:

```rust
# use bitflags::bitflags;
bitflags! {
    // Attributes can be applied to flags types
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Flags: u32 {
        const A = 0b00000001;
        const B = 0b00000010;
        const C = 0b00000100;
    }
}

// Impl blocks can be added to flags types
impl Flags {
    pub fn as_u64(&self) -> u64 {
        self.bits() as u64
    }
}
```

## Working with flags values

Use generated constants and standard bitwise operators to interact with flags values:

```rust
# use bitflags::bitflags;
# bitflags! {
#     #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#     pub struct Flags: u32 {
#         const A = 0b00000001;
#         const B = 0b00000010;
#         const C = 0b00000100;
#     }
# }
// union
let ab = Flags::A | Flags::B;

// intersection
let a = ab & Flags::A;

// difference
let b = ab - Flags::A;

// complement
let c = !ab;
```

See the docs for the [`Flags`] trait for more details on operators and how they behave.

# Formatting and parsing

`bitflags` defines a text format that can be used to convert any flags value to and from strings.

See the [`parser`] module for more details.

# Specification

The terminology and behavior of generated flags types is
[specified in the source repository](https://github.com/bitflags/bitflags/blob/main/spec.md).
Details are repeated in these docs where appropriate, but is exhaustively listed in the spec. Some
things are worth calling out explicitly here.

## Flags types, flags values, flags

The spec and these docs use consistent terminology to refer to things in the bitflags domain:

- **Bits type**: A type that defines a fixed number of bits at specific locations.
- **Flag**: A set of bits in a bits type that may have a unique name.
- **Flags type**: A set of defined flags over a specific bits type.
- **Flags value**: An instance of a flags type using its specific bits value for storage.

```
# use bitflags::bitflags;
bitflags! {
    struct FlagsType: u8 {
//                    -- Bits type
//         --------- Flags type
        const A = 1;
//            ----- Flag
    }
}

let flag = FlagsType::A;
//  ---- Flags value
```

## Known and unknown bits

Any bits in a flag you define are called _known bits_. Any other bits are _unknown bits_.
In the following flags type:

```
# use bitflags::bitflags;
bitflags! {
    struct Flags: u8 {
        const A = 1;
        const B = 1 << 1;
        const C = 1 << 2;
    }
}
```

The known bits are `0b0000_0111` and the unknown bits are `0b1111_1000`.

`bitflags` doesn't guarantee that a flags value will only ever have known bits set, but some operators
will unset any unknown bits they encounter. In a future version of `bitflags`, all operators will
unset unknown bits.

If you're using `bitflags` for flags types defined externally, such as from C, you probably want all
bits to be considered known, in case that external source changes. You can do this using an unnamed
flag, as described in [externally defined flags](#externally-defined-flags).

## Zero-bit flags

Flags with no bits set should be avoided because they interact strangely with [`Flags::contains`]
and [`Flags::intersects`]. A zero-bit flag is always contained, but is never intersected. The
names of zero-bit flags can be parsed, but are never formatted.

## Multi-bit flags

Flags that set multiple bits should be avoided unless each bit is also in a single-bit flag.
Take the following flags type as an example:

```
# use bitflags::bitflags;
bitflags! {
    struct Flags: u8 {
        const A = 1;
        const B = 1 | 1 << 1;
    }
}
```

The result of `Flags::A ^ Flags::B` is `0b0000_0010`, which doesn't correspond to either
`Flags::A` or `Flags::B` even though it's still a known bit.
*/
#![allow(mixed_script_confusables)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;
#[doc(inline)]
pub use traits::{Bits, Flag, Flags};
pub mod iter {
    /*!
Yield the bits of a source flags value in a set of contained flags values.
*/
    use crate::{Flag, Flags};
    /**
An iterator over flags values.

This iterator will yield flags values for contained, defined flags first, with any remaining bits yielded
as a final flags value.
*/
    pub struct Iter<B: 'static> {
        inner: IterNames<B>,
        done: bool,
    }
    impl<B: Flags> Iter<B> {
        pub(crate) fn new(flags: &B) -> Self {
            Iter {
                inner: IterNames::new(flags),
                done: false,
            }
        }
    }
    impl<B: 'static> Iter<B> {
        #[doc(hidden)]
        pub const fn __private_const_new(
            flags: &'static [Flag<B>],
            source: B,
            remaining: B,
        ) -> Self {
            Iter {
                inner: IterNames::__private_const_new(flags, source, remaining),
                done: false,
            }
        }
    }
    impl<B: Flags> Iterator for Iter<B> {
        type Item = B;
        fn next(&mut self) -> Option<Self::Item> {
            match self.inner.next() {
                Some((_, flag)) => Some(flag),
                None if !self.done => {
                    self.done = true;
                    if !self.inner.remaining().is_empty() {
                        Some(B::from_bits_retain(self.inner.remaining.bits()))
                    } else {
                        None
                    }
                }
                None => None,
            }
        }
    }
    /**
An iterator over flags values.

This iterator only yields flags values for contained, defined, named flags. Any remaining bits
won't be yielded, but can be found with the [`IterNames::remaining`] method.
*/
    pub struct IterNames<B: 'static> {
        flags: &'static [Flag<B>],
        idx: usize,
        source: B,
        remaining: B,
    }
    impl<B: Flags> IterNames<B> {
        pub(crate) fn new(flags: &B) -> Self {
            IterNames {
                flags: B::FLAGS,
                idx: 0,
                remaining: B::from_bits_retain(flags.bits()),
                source: B::from_bits_retain(flags.bits()),
            }
        }
    }
    impl<B: 'static> IterNames<B> {
        #[doc(hidden)]
        pub const fn __private_const_new(
            flags: &'static [Flag<B>],
            source: B,
            remaining: B,
        ) -> Self {
            IterNames {
                flags,
                idx: 0,
                remaining,
                source,
            }
        }
        /// Get a flags value of any remaining bits that haven't been yielded yet.
        ///
        /// Once the iterator has finished, this method can be used to
        /// check whether or not there are any bits that didn't correspond
        /// to a contained, defined, named flag remaining.
        pub fn remaining(&self) -> &B {
            &self.remaining
        }
    }
    impl<B: Flags> Iterator for IterNames<B> {
        type Item = (&'static str, B);
        fn next(&mut self) -> Option<Self::Item> {
            while let Some(flag) = self.flags.get(self.idx) {
                if self.remaining.is_empty() {
                    return None;
                }
                self.idx += 1;
                if flag.name().is_empty() {
                    continue;
                }
                let bits = flag.value().bits();
                if self.source.contains(B::from_bits_retain(bits))
                    && self.remaining.intersects(B::from_bits_retain(bits))
                {
                    self.remaining.remove(B::from_bits_retain(bits));
                    return Some((flag.name(), B::from_bits_retain(bits)));
                }
            }
            None
        }
    }
}
pub mod parser {
    /*!
Parsing flags from text.

Format and parse a flags value as text using the following grammar:

- _Flags:_ (_Whitespace_ _Flag_ _Whitespace_)`|`*
- _Flag:_ _Name_ | _Hex Number_
- _Name:_ The name of any defined flag
- _Hex Number_: `0x`([0-9a-fA-F])*
- _Whitespace_: (\s)*

As an example, this is how `Flags::A | Flags::B | 0x0c` can be represented as text:

```text
A | B | 0x0c
```

Alternatively, it could be represented without whitespace:

```text
A|B|0x0C
```

Note that identifiers are *case-sensitive*, so the following is *not equivalent*:

```text
a|b|0x0C
```
*/
    #![allow(clippy::let_unit_value)]
    use core::fmt::{self, Write};
    use crate::{Bits, Flags};
    /**
Write a flags value as text.

Any bits that aren't part of a contained flag will be formatted as a hex number.
*/
    pub fn to_writer<B: Flags>(
        flags: &B,
        mut writer: impl Write,
    ) -> Result<(), fmt::Error>
    where
        B::Bits: WriteHex,
    {
        let mut first = true;
        let mut iter = flags.iter_names();
        for (name, _) in &mut iter {
            if !first {
                writer.write_str(" | ")?;
            }
            first = false;
            writer.write_str(name)?;
        }
        let remaining = iter.remaining().bits();
        if remaining != B::Bits::EMPTY {
            if !first {
                writer.write_str(" | ")?;
            }
            writer.write_str("0x")?;
            remaining.write_hex(writer)?;
        }
        fmt::Result::Ok(())
    }
    /**
Parse a flags value from text.

This function will fail on any names that don't correspond to defined flags.
Unknown bits will be retained.
*/
    pub fn from_str<B: Flags>(input: &str) -> Result<B, ParseError>
    where
        B::Bits: ParseHex,
    {
        let mut parsed_flags = B::empty();
        if input.trim().is_empty() {
            return Ok(parsed_flags);
        }
        for flag in input.split('|') {
            let flag = flag.trim();
            if flag.is_empty() {
                return Err(ParseError::empty_flag());
            }
            let parsed_flag = if let Some(flag) = flag.strip_prefix("0x") {
                let bits = <B::Bits>::parse_hex(flag)
                    .map_err(|_| ParseError::invalid_hex_flag(flag))?;
                B::from_bits_retain(bits)
            } else {
                B::from_name(flag).ok_or_else(|| ParseError::invalid_named_flag(flag))?
            };
            parsed_flags.insert(parsed_flag);
        }
        Ok(parsed_flags)
    }
    /**
Write a flags value as text, ignoring any unknown bits.
*/
    pub fn to_writer_truncate<B: Flags>(
        flags: &B,
        writer: impl Write,
    ) -> Result<(), fmt::Error>
    where
        B::Bits: WriteHex,
    {
        to_writer(&B::from_bits_truncate(flags.bits()), writer)
    }
    /**
Parse a flags value from text.

This function will fail on any names that don't correspond to defined flags.
Unknown bits will be ignored.
*/
    pub fn from_str_truncate<B: Flags>(input: &str) -> Result<B, ParseError>
    where
        B::Bits: ParseHex,
    {
        Ok(B::from_bits_truncate(from_str::<B>(input)?.bits()))
    }
    /**
Write only the contained, defined, named flags in a flags value as text.
*/
    pub fn to_writer_strict<B: Flags>(
        flags: &B,
        mut writer: impl Write,
    ) -> Result<(), fmt::Error> {
        let mut first = true;
        let mut iter = flags.iter_names();
        for (name, _) in &mut iter {
            if !first {
                writer.write_str(" | ")?;
            }
            first = false;
            writer.write_str(name)?;
        }
        fmt::Result::Ok(())
    }
    /**
Parse a flags value from text.

This function will fail on any names that don't correspond to defined flags.
This function will fail to parse hex values.
*/
    pub fn from_str_strict<B: Flags>(input: &str) -> Result<B, ParseError> {
        let mut parsed_flags = B::empty();
        if input.trim().is_empty() {
            return Ok(parsed_flags);
        }
        for flag in input.split('|') {
            let flag = flag.trim();
            if flag.is_empty() {
                return Err(ParseError::empty_flag());
            }
            if flag.starts_with("0x") {
                return Err(ParseError::invalid_hex_flag("unsupported hex flag value"));
            }
            let parsed_flag = B::from_name(flag)
                .ok_or_else(|| ParseError::invalid_named_flag(flag))?;
            parsed_flags.insert(parsed_flag);
        }
        Ok(parsed_flags)
    }
    /**
Encode a value as a hex string.

Implementors of this trait should not write the `0x` prefix.
*/
    pub trait WriteHex {
        /// Write the value as hex.
        fn write_hex<W: fmt::Write>(&self, writer: W) -> fmt::Result;
    }
    /**
Parse a value from a hex string.
*/
    pub trait ParseHex {
        /// Parse the value from hex.
        fn parse_hex(input: &str) -> Result<Self, ParseError>
        where
            Self: Sized;
    }
    /// An error encountered while parsing flags from text.
    pub struct ParseError(ParseErrorKind);
    #[automatically_derived]
    impl ::core::fmt::Debug for ParseError {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_tuple_field1_finish(f, "ParseError", &&self.0)
        }
    }
    #[allow(clippy::enum_variant_names)]
    enum ParseErrorKind {
        EmptyFlag,
        InvalidNamedFlag { got: () },
        InvalidHexFlag { got: () },
    }
    #[automatically_derived]
    #[allow(clippy::enum_variant_names)]
    impl ::core::fmt::Debug for ParseErrorKind {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                ParseErrorKind::EmptyFlag => {
                    ::core::fmt::Formatter::write_str(f, "EmptyFlag")
                }
                ParseErrorKind::InvalidNamedFlag { got: __self_0 } => {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "InvalidNamedFlag",
                        "got",
                        &__self_0,
                    )
                }
                ParseErrorKind::InvalidHexFlag { got: __self_0 } => {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "InvalidHexFlag",
                        "got",
                        &__self_0,
                    )
                }
            }
        }
    }
    impl ParseError {
        /// An invalid hex flag was encountered.
        pub fn invalid_hex_flag(flag: impl fmt::Display) -> Self {
            let _flag = flag;
            let got = {};
            ParseError(ParseErrorKind::InvalidHexFlag {
                got,
            })
        }
        /// A named flag that doesn't correspond to any on the flags type was encountered.
        pub fn invalid_named_flag(flag: impl fmt::Display) -> Self {
            let _flag = flag;
            let got = {};
            ParseError(ParseErrorKind::InvalidNamedFlag {
                got,
            })
        }
        /// A hex or named flag wasn't found between separators.
        pub const fn empty_flag() -> Self {
            ParseError(ParseErrorKind::EmptyFlag)
        }
    }
    impl fmt::Display for ParseError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match &self.0 {
                ParseErrorKind::InvalidNamedFlag { got } => {
                    let _got = got;
                    f.write_fmt(format_args!("unrecognized named flag"))?;
                }
                ParseErrorKind::InvalidHexFlag { got } => {
                    let _got = got;
                    f.write_fmt(format_args!("invalid hex flag"))?;
                }
                ParseErrorKind::EmptyFlag => {
                    f.write_fmt(format_args!("encountered empty flag"))?;
                }
            }
            Ok(())
        }
    }
}
mod traits {
    use core::{fmt, ops::{BitAnd, BitOr, BitXor, Not}};
    use crate::{iter, parser::{ParseError, ParseHex, WriteHex}};
    /**
A defined flags value that may be named or unnamed.
*/
    pub struct Flag<B> {
        name: &'static str,
        value: B,
    }
    #[automatically_derived]
    impl<B: ::core::fmt::Debug> ::core::fmt::Debug for Flag<B> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "Flag",
                "name",
                &self.name,
                "value",
                &&self.value,
            )
        }
    }
    impl<B> Flag<B> {
        /**
    Define a flag.

    If `name` is non-empty then the flag is named, otherwise it's unnamed.
    */
        pub const fn new(name: &'static str, value: B) -> Self {
            Flag { name, value }
        }
        /**
    Get the name of this flag.

    If the flag is unnamed then the returned string will be empty.
    */
        pub const fn name(&self) -> &'static str {
            self.name
        }
        /**
    Get the flags value of this flag.
    */
        pub const fn value(&self) -> &B {
            &self.value
        }
        /**
    Whether the flag is named.

    If [`Flag::name`] returns a non-empty string then this method will return `true`.
    */
        pub const fn is_named(&self) -> bool {
            !self.name.is_empty()
        }
        /**
    Whether the flag is unnamed.

    If [`Flag::name`] returns a non-empty string then this method will return `false`.
    */
        pub const fn is_unnamed(&self) -> bool {
            self.name.is_empty()
        }
    }
    /**
A set of defined flags using a bits type as storage.

## Implementing `Flags`

This trait is implemented by the [`bitflags`](macro.bitflags.html) macro:

```
use bitflags::bitflags;

bitflags! {
    struct MyFlags: u8 {
        const A = 1;
        const B = 1 << 1;
    }
}
```

It can also be implemented manually:

```
use bitflags::{Flag, Flags};

struct MyFlags(u8);

impl Flags for MyFlags {
    const FLAGS: &'static [Flag<Self>] = &[
        Flag::new("A", MyFlags(1)),
        Flag::new("B", MyFlags(1 << 1)),
    ];

    type Bits = u8;

    fn from_bits_retain(bits: Self::Bits) -> Self {
        MyFlags(bits)
    }

    fn bits(&self) -> Self::Bits {
        self.0
    }
}
```

## Using `Flags`

The `Flags` trait can be used generically to work with any flags types. In this example,
we can count the number of defined named flags:

```
# use bitflags::{bitflags, Flags};
fn defined_flags<F: Flags>() -> usize {
    F::FLAGS.iter().filter(|f| f.is_named()).count()
}

bitflags! {
    struct MyFlags: u8 {
        const A = 1;
        const B = 1 << 1;
        const C = 1 << 2;

        const _ = !0;
    }
}

assert_eq!(3, defined_flags::<MyFlags>());
```
*/
    pub trait Flags: Sized + 'static {
        /// The set of defined flags.
        const FLAGS: &'static [Flag<Self>];
        /// The underlying bits type.
        type Bits: Bits;
        /// Get a flags value with all bits unset.
        fn empty() -> Self {
            Self::from_bits_retain(Self::Bits::EMPTY)
        }
        /// Get a flags value with all known bits set.
        fn all() -> Self {
            let mut truncated = Self::Bits::EMPTY;
            for flag in Self::FLAGS.iter() {
                truncated = truncated | flag.value().bits();
            }
            Self::from_bits_retain(truncated)
        }
        /// Get the underlying bits value.
        ///
        /// The returned value is exactly the bits set in this flags value.
        fn bits(&self) -> Self::Bits;
        /// Convert from a bits value.
        ///
        /// This method will return `None` if any unknown bits are set.
        fn from_bits(bits: Self::Bits) -> Option<Self> {
            let truncated = Self::from_bits_truncate(bits);
            if truncated.bits() == bits { Some(truncated) } else { None }
        }
        /// Convert from a bits value, unsetting any unknown bits.
        fn from_bits_truncate(bits: Self::Bits) -> Self {
            Self::from_bits_retain(bits & Self::all().bits())
        }
        /// Convert from a bits value exactly.
        fn from_bits_retain(bits: Self::Bits) -> Self;
        /// Get a flags value with the bits of a flag with the given name set.
        ///
        /// This method will return `None` if `name` is empty or doesn't
        /// correspond to any named flag.
        fn from_name(name: &str) -> Option<Self> {
            if name.is_empty() {
                return None;
            }
            for flag in Self::FLAGS {
                if flag.name() == name {
                    return Some(Self::from_bits_retain(flag.value().bits()));
                }
            }
            None
        }
        /// Yield a set of contained flags values.
        ///
        /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
        /// will be yielded together as a final flags value.
        fn iter(&self) -> iter::Iter<Self> {
            iter::Iter::new(self)
        }
        /// Yield a set of contained named flags values.
        ///
        /// This method is like [`Flags::iter`], except only yields bits in contained named flags.
        /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
        fn iter_names(&self) -> iter::IterNames<Self> {
            iter::IterNames::new(self)
        }
        /// Whether all bits in this flags value are unset.
        fn is_empty(&self) -> bool {
            self.bits() == Self::Bits::EMPTY
        }
        /// Whether all known bits in this flags value are set.
        fn is_all(&self) -> bool {
            Self::all().bits() | self.bits() == self.bits()
        }
        /// Whether any set bits in a source flags value are also set in a target flags value.
        fn intersects(&self, other: Self) -> bool
        where
            Self: Sized,
        {
            self.bits() & other.bits() != Self::Bits::EMPTY
        }
        /// Whether all set bits in a source flags value are also set in a target flags value.
        fn contains(&self, other: Self) -> bool
        where
            Self: Sized,
        {
            self.bits() & other.bits() == other.bits()
        }
        /// The bitwise or (`|`) of the bits in two flags values.
        fn insert(&mut self, other: Self)
        where
            Self: Sized,
        {
            *self = Self::from_bits_retain(self.bits()).union(other);
        }
        /// The intersection of a source flags value with the complement of a target flags value (`&!`).
        ///
        /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
        /// `remove` won't truncate `other`, but the `!` operator will.
        fn remove(&mut self, other: Self)
        where
            Self: Sized,
        {
            *self = Self::from_bits_retain(self.bits()).difference(other);
        }
        /// The bitwise exclusive-or (`^`) of the bits in two flags values.
        fn toggle(&mut self, other: Self)
        where
            Self: Sized,
        {
            *self = Self::from_bits_retain(self.bits()).symmetric_difference(other);
        }
        /// Call [`Flags::insert`] when `value` is `true` or [`Flags::remove`] when `value` is `false`.
        fn set(&mut self, other: Self, value: bool)
        where
            Self: Sized,
        {
            if value {
                self.insert(other);
            } else {
                self.remove(other);
            }
        }
        /// The bitwise and (`&`) of the bits in two flags values.
        #[must_use]
        fn intersection(self, other: Self) -> Self {
            Self::from_bits_retain(self.bits() & other.bits())
        }
        /// The bitwise or (`|`) of the bits in two flags values.
        #[must_use]
        fn union(self, other: Self) -> Self {
            Self::from_bits_retain(self.bits() | other.bits())
        }
        /// The intersection of a source flags value with the complement of a target flags value (`&!`).
        ///
        /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
        /// `difference` won't truncate `other`, but the `!` operator will.
        #[must_use]
        fn difference(self, other: Self) -> Self {
            Self::from_bits_retain(self.bits() & !other.bits())
        }
        /// The bitwise exclusive-or (`^`) of the bits in two flags values.
        #[must_use]
        fn symmetric_difference(self, other: Self) -> Self {
            Self::from_bits_retain(self.bits() ^ other.bits())
        }
        /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
        #[must_use]
        fn complement(self) -> Self {
            Self::from_bits_truncate(!self.bits())
        }
    }
    /**
A bits type that can be used as storage for a flags type.
*/
    pub trait Bits: Clone + Copy + PartialEq + BitAnd<
            Output = Self,
        > + BitOr<
            Output = Self,
        > + BitXor<Output = Self> + Not<Output = Self> + Sized + 'static {
        /// A value with all bits unset.
        const EMPTY: Self;
        /// A value with all bits set.
        const ALL: Self;
    }
    pub trait Primitive {}
    impl Bits for u8 {
        const EMPTY: u8 = 0;
        const ALL: u8 = <u8>::MAX;
    }
    impl Bits for i8 {
        const EMPTY: i8 = 0;
        const ALL: i8 = <u8>::MAX as i8;
    }
    impl ParseHex for u8 {
        fn parse_hex(input: &str) -> Result<Self, ParseError> {
            <u8>::from_str_radix(input, 16)
                .map_err(|_| ParseError::invalid_hex_flag(input))
        }
    }
    impl ParseHex for i8 {
        fn parse_hex(input: &str) -> Result<Self, ParseError> {
            <i8>::from_str_radix(input, 16)
                .map_err(|_| ParseError::invalid_hex_flag(input))
        }
    }
    impl WriteHex for u8 {
        fn write_hex<W: fmt::Write>(&self, mut writer: W) -> fmt::Result {
            writer.write_fmt(format_args!("{0:x}", self))
        }
    }
    impl WriteHex for i8 {
        fn write_hex<W: fmt::Write>(&self, mut writer: W) -> fmt::Result {
            writer.write_fmt(format_args!("{0:x}", self))
        }
    }
    impl Primitive for i8 {}
    impl Primitive for u8 {}
    impl Bits for u16 {
        const EMPTY: u16 = 0;
        const ALL: u16 = <u16>::MAX;
    }
    impl Bits for i16 {
        const EMPTY: i16 = 0;
        const ALL: i16 = <u16>::MAX as i16;
    }
    impl ParseHex for u16 {
        fn parse_hex(input: &str) -> Result<Self, ParseError> {
            <u16>::from_str_radix(input, 16)
                .map_err(|_| ParseError::invalid_hex_flag(input))
        }
    }
    impl ParseHex for i16 {
        fn parse_hex(input: &str) -> Result<Self, ParseError> {
            <i16>::from_str_radix(input, 16)
                .map_err(|_| ParseError::invalid_hex_flag(input))
        }
    }
    impl WriteHex for u16 {
        fn write_hex<W: fmt::Write>(&self, mut writer: W) -> fmt::Result {
            writer.write_fmt(format_args!("{0:x}", self))
        }
    }
    impl WriteHex for i16 {
        fn write_hex<W: fmt::Write>(&self, mut writer: W) -> fmt::Result {
            writer.write_fmt(format_args!("{0:x}", self))
        }
    }
    impl Primitive for i16 {}
    impl Primitive for u16 {}
    impl Bits for u32 {
        const EMPTY: u32 = 0;
        const ALL: u32 = <u32>::MAX;
    }
    impl Bits for i32 {
        const EMPTY: i32 = 0;
        const ALL: i32 = <u32>::MAX as i32;
    }
    impl ParseHex for u32 {
        fn parse_hex(input: &str) -> Result<Self, ParseError> {
            <u32>::from_str_radix(input, 16)
                .map_err(|_| ParseError::invalid_hex_flag(input))
        }
    }
    impl ParseHex for i32 {
        fn parse_hex(input: &str) -> Result<Self, ParseError> {
            <i32>::from_str_radix(input, 16)
                .map_err(|_| ParseError::invalid_hex_flag(input))
        }
    }
    impl WriteHex for u32 {
        fn write_hex<W: fmt::Write>(&self, mut writer: W) -> fmt::Result {
            writer.write_fmt(format_args!("{0:x}", self))
        }
    }
    impl WriteHex for i32 {
        fn write_hex<W: fmt::Write>(&self, mut writer: W) -> fmt::Result {
            writer.write_fmt(format_args!("{0:x}", self))
        }
    }
    impl Primitive for i32 {}
    impl Primitive for u32 {}
    impl Bits for u64 {
        const EMPTY: u64 = 0;
        const ALL: u64 = <u64>::MAX;
    }
    impl Bits for i64 {
        const EMPTY: i64 = 0;
        const ALL: i64 = <u64>::MAX as i64;
    }
    impl ParseHex for u64 {
        fn parse_hex(input: &str) -> Result<Self, ParseError> {
            <u64>::from_str_radix(input, 16)
                .map_err(|_| ParseError::invalid_hex_flag(input))
        }
    }
    impl ParseHex for i64 {
        fn parse_hex(input: &str) -> Result<Self, ParseError> {
            <i64>::from_str_radix(input, 16)
                .map_err(|_| ParseError::invalid_hex_flag(input))
        }
    }
    impl WriteHex for u64 {
        fn write_hex<W: fmt::Write>(&self, mut writer: W) -> fmt::Result {
            writer.write_fmt(format_args!("{0:x}", self))
        }
    }
    impl WriteHex for i64 {
        fn write_hex<W: fmt::Write>(&self, mut writer: W) -> fmt::Result {
            writer.write_fmt(format_args!("{0:x}", self))
        }
    }
    impl Primitive for i64 {}
    impl Primitive for u64 {}
    impl Bits for u128 {
        const EMPTY: u128 = 0;
        const ALL: u128 = <u128>::MAX;
    }
    impl Bits for i128 {
        const EMPTY: i128 = 0;
        const ALL: i128 = <u128>::MAX as i128;
    }
    impl ParseHex for u128 {
        fn parse_hex(input: &str) -> Result<Self, ParseError> {
            <u128>::from_str_radix(input, 16)
                .map_err(|_| ParseError::invalid_hex_flag(input))
        }
    }
    impl ParseHex for i128 {
        fn parse_hex(input: &str) -> Result<Self, ParseError> {
            <i128>::from_str_radix(input, 16)
                .map_err(|_| ParseError::invalid_hex_flag(input))
        }
    }
    impl WriteHex for u128 {
        fn write_hex<W: fmt::Write>(&self, mut writer: W) -> fmt::Result {
            writer.write_fmt(format_args!("{0:x}", self))
        }
    }
    impl WriteHex for i128 {
        fn write_hex<W: fmt::Write>(&self, mut writer: W) -> fmt::Result {
            writer.write_fmt(format_args!("{0:x}", self))
        }
    }
    impl Primitive for i128 {}
    impl Primitive for u128 {}
    impl Bits for usize {
        const EMPTY: usize = 0;
        const ALL: usize = <usize>::MAX;
    }
    impl Bits for isize {
        const EMPTY: isize = 0;
        const ALL: isize = <usize>::MAX as isize;
    }
    impl ParseHex for usize {
        fn parse_hex(input: &str) -> Result<Self, ParseError> {
            <usize>::from_str_radix(input, 16)
                .map_err(|_| ParseError::invalid_hex_flag(input))
        }
    }
    impl ParseHex for isize {
        fn parse_hex(input: &str) -> Result<Self, ParseError> {
            <isize>::from_str_radix(input, 16)
                .map_err(|_| ParseError::invalid_hex_flag(input))
        }
    }
    impl WriteHex for usize {
        fn write_hex<W: fmt::Write>(&self, mut writer: W) -> fmt::Result {
            writer.write_fmt(format_args!("{0:x}", self))
        }
    }
    impl WriteHex for isize {
        fn write_hex<W: fmt::Write>(&self, mut writer: W) -> fmt::Result {
            writer.write_fmt(format_args!("{0:x}", self))
        }
    }
    impl Primitive for isize {}
    impl Primitive for usize {}
    /// A trait for referencing the `bitflags`-owned internal type
    /// without exposing it publicly.
    pub trait PublicFlags {
        /// The type of the underlying storage.
        type Primitive: Primitive;
        /// The type of the internal field on the generated flags type.
        type Internal;
    }
    #[doc(hidden)]
    #[deprecated(note = "use the `Flags` trait instead")]
    pub trait BitFlags: ImplementedByBitFlagsMacro + Flags {
        /// An iterator over enabled flags in an instance of the type.
        type Iter: Iterator<Item = Self>;
        /// An iterator over the raw names and bits for enabled flags in an instance of the type.
        type IterNames: Iterator<Item = (&'static str, Self)>;
    }
    #[allow(deprecated)]
    impl<B: Flags> BitFlags for B {
        type Iter = iter::Iter<Self>;
        type IterNames = iter::IterNames<Self>;
    }
    impl<B: Flags> ImplementedByBitFlagsMacro for B {}
    /// A marker trait that signals that an implementation of `BitFlags` came from the `bitflags!` macro.
    ///
    /// There's nothing stopping an end-user from implementing this trait, but we don't guarantee their
    /// manual implementations won't break between non-breaking releases.
    #[doc(hidden)]
    pub trait ImplementedByBitFlagsMacro {}
    pub(crate) mod __private {
        pub use super::{ImplementedByBitFlagsMacro, PublicFlags};
    }
}
#[doc(hidden)]
pub mod __private {
    #[allow(unused_imports)]
    pub use crate::{external::__private::*, traits::__private::*};
    pub use core;
}
#[allow(unused_imports)]
pub use external::*;
#[allow(deprecated)]
pub use traits::BitFlags;
#[macro_use]
mod public {
    //! Generate the user-facing flags type.
    //!
    //! The code here belongs to the end-user, so new trait implementations and methods can't be
    //! added without potentially breaking users.
}
#[macro_use]
mod internal {
    //! Generate the internal `bitflags`-facing flags type.
    //!
    //! The code generated here is owned by `bitflags`, but still part of its public API.
    //! Changes to the types generated here need to be considered like any other public API change.
}
#[macro_use]
mod external {
    //! Conditional trait implementations for external libraries.
    pub(crate) mod __private {}
}
mod tests {
    mod all {
        use super::*;
        use crate::Flags;
        extern crate test;
        #[rustc_test_marker = "tests::all::cases"]
        #[doc(hidden)]
        pub const cases: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::all::cases"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/all.rs",
                start_line: 6usize,
                start_col: 4usize,
                end_line: 6usize,
                end_col: 9usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(cases()),
            ),
        };
        fn cases() {
            case(1 | 1 << 1 | 1 << 2, TestFlags::all);
            case(0, TestZero::all);
            case(0, TestEmpty::all);
            case(!0, TestExternal::all);
        }
        #[track_caller]
        fn case<T: Flags>(expected: T::Bits, inherent: impl FnOnce() -> T)
        where
            <T as Flags>::Bits: std::fmt::Debug + PartialEq,
        {
            match (&expected, &inherent().bits()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(format_args!("T::all()")),
                        );
                    }
                }
            };
            match (&expected, &T::all().bits()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(format_args!("Flags::all()")),
                        );
                    }
                }
            };
        }
    }
    mod bits {
        use super::*;
        use crate::Flags;
        extern crate test;
        #[rustc_test_marker = "tests::bits::cases"]
        #[doc(hidden)]
        pub const cases: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::bits::cases"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/bits.rs",
                start_line: 6usize,
                start_col: 4usize,
                end_line: 6usize,
                end_col: 9usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(cases()),
            ),
        };
        fn cases() {
            case(0, TestFlags::empty(), TestFlags::bits);
            case(1, TestFlags::A, TestFlags::bits);
            case(1 | 1 << 1 | 1 << 2, TestFlags::ABC, TestFlags::bits);
            case(!0, TestFlags::from_bits_retain(u8::MAX), TestFlags::bits);
            case(1 << 3, TestFlags::from_bits_retain(1 << 3), TestFlags::bits);
            case(1 << 3, TestZero::from_bits_retain(1 << 3), TestZero::bits);
            case(1 << 3, TestEmpty::from_bits_retain(1 << 3), TestEmpty::bits);
            case(
                1 << 4 | 1 << 6,
                TestExternal::from_bits_retain(1 << 4 | 1 << 6),
                TestExternal::bits,
            );
        }
        #[track_caller]
        fn case<T: Flags + std::fmt::Debug>(
            expected: T::Bits,
            value: T,
            inherent: impl FnOnce(&T) -> T::Bits,
        )
        where
            T::Bits: std::fmt::Debug + PartialEq,
        {
            match (&expected, &inherent(&value)) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("{0:?}.bits()", value),
                            ),
                        );
                    }
                }
            };
            match (&expected, &Flags::bits(&value)) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("Flags::bits({0:?})", value),
                            ),
                        );
                    }
                }
            };
        }
    }
    mod complement {
        use super::*;
        use crate::Flags;
        extern crate test;
        #[rustc_test_marker = "tests::complement::cases"]
        #[doc(hidden)]
        pub const cases: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::complement::cases"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/complement.rs",
                start_line: 6usize,
                start_col: 4usize,
                end_line: 6usize,
                end_col: 9usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(cases()),
            ),
        };
        fn cases() {
            case(0, TestFlags::all(), TestFlags::complement);
            case(0, TestFlags::from_bits_retain(!0), TestFlags::complement);
            case(1 | 1 << 1, TestFlags::C, TestFlags::complement);
            case(
                1 | 1 << 1,
                TestFlags::C | TestFlags::from_bits_retain(1 << 3),
                TestFlags::complement,
            );
            case(1 | 1 << 1 | 1 << 2, TestFlags::empty(), TestFlags::complement);
            case(
                1 | 1 << 1 | 1 << 2,
                TestFlags::from_bits_retain(1 << 3),
                TestFlags::complement,
            );
            case(0, TestZero::empty(), TestZero::complement);
            case(0, TestEmpty::empty(), TestEmpty::complement);
            case(1 << 2, TestOverlapping::AB, TestOverlapping::complement);
            case(!0, TestExternal::empty(), TestExternal::complement);
        }
        #[track_caller]
        fn case<T: Flags + std::fmt::Debug + std::ops::Not<Output = T> + Copy>(
            expected: T::Bits,
            value: T,
            inherent: impl FnOnce(T) -> T,
        )
        where
            T::Bits: std::fmt::Debug + PartialEq,
        {
            match (&expected, &inherent(value).bits()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("{0:?}.complement()", value),
                            ),
                        );
                    }
                }
            };
            match (&expected, &Flags::complement(value).bits()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("Flags::complement({0:?})", value),
                            ),
                        );
                    }
                }
            };
            match (&expected, &(!value).bits()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(format_args!("!{0:?}", value)),
                        );
                    }
                }
            };
        }
    }
    mod contains {
        use super::*;
        use crate::Flags;
        extern crate test;
        #[rustc_test_marker = "tests::contains::cases"]
        #[doc(hidden)]
        pub const cases: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::contains::cases"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/contains.rs",
                start_line: 6usize,
                start_col: 4usize,
                end_line: 6usize,
                end_col: 9usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(cases()),
            ),
        };
        fn cases() {
            case(
                TestFlags::empty(),
                &[
                    (TestFlags::empty(), true),
                    (TestFlags::A, false),
                    (TestFlags::B, false),
                    (TestFlags::C, false),
                    (TestFlags::from_bits_retain(1 << 3), false),
                ],
                TestFlags::contains,
            );
            case(
                TestFlags::A,
                &[
                    (TestFlags::empty(), true),
                    (TestFlags::A, true),
                    (TestFlags::B, false),
                    (TestFlags::C, false),
                    (TestFlags::ABC, false),
                    (TestFlags::from_bits_retain(1 << 3), false),
                    (TestFlags::from_bits_retain(1 | (1 << 3)), false),
                ],
                TestFlags::contains,
            );
            case(
                TestFlags::ABC,
                &[
                    (TestFlags::empty(), true),
                    (TestFlags::A, true),
                    (TestFlags::B, true),
                    (TestFlags::C, true),
                    (TestFlags::ABC, true),
                    (TestFlags::from_bits_retain(1 << 3), false),
                ],
                TestFlags::contains,
            );
            case(
                TestFlags::from_bits_retain(1 << 3),
                &[
                    (TestFlags::empty(), true),
                    (TestFlags::A, false),
                    (TestFlags::B, false),
                    (TestFlags::C, false),
                    (TestFlags::from_bits_retain(1 << 3), true),
                ],
                TestFlags::contains,
            );
            case(TestZero::ZERO, &[(TestZero::ZERO, true)], TestZero::contains);
            case(
                TestOverlapping::AB,
                &[
                    (TestOverlapping::AB, true),
                    (TestOverlapping::BC, false),
                    (TestOverlapping::from_bits_retain(1 << 1), true),
                ],
                TestOverlapping::contains,
            );
            case(
                TestExternal::all(),
                &[
                    (TestExternal::A, true),
                    (TestExternal::B, true),
                    (TestExternal::C, true),
                    (TestExternal::from_bits_retain(1 << 5 | 1 << 7), true),
                ],
                TestExternal::contains,
            );
        }
        #[track_caller]
        fn case<T: Flags + std::fmt::Debug + Copy>(
            value: T,
            inputs: &[(T, bool)],
            mut inherent: impl FnMut(&T, T) -> bool,
        ) {
            for (input, expected) in inputs {
                match (&*expected, &inherent(&value, *input)) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("{0:?}.contains({1:?})", value, input),
                                ),
                            );
                        }
                    }
                };
                match (&*expected, &Flags::contains(&value, *input)) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("Flags::contains({0:?}, {1:?})", value, input),
                                ),
                            );
                        }
                    }
                };
            }
        }
    }
    mod difference {
        use super::*;
        use crate::Flags;
        extern crate test;
        #[rustc_test_marker = "tests::difference::cases"]
        #[doc(hidden)]
        pub const cases: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::difference::cases"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/difference.rs",
                start_line: 6usize,
                start_col: 4usize,
                end_line: 6usize,
                end_col: 9usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(cases()),
            ),
        };
        fn cases() {
            case(
                TestFlags::A | TestFlags::B,
                &[
                    (TestFlags::A, 1 << 1),
                    (TestFlags::B, 1),
                    (TestFlags::from_bits_retain(1 << 3), 1 | 1 << 1),
                ],
                TestFlags::difference,
            );
            case(
                TestFlags::from_bits_retain(1 | 1 << 3),
                &[(TestFlags::A, 1 << 3), (TestFlags::from_bits_retain(1 << 3), 1)],
                TestFlags::difference,
            );
            case(
                TestExternal::from_bits_retain(!0),
                &[(TestExternal::A, 0b1111_1110)],
                TestExternal::difference,
            );
            match (
                &0b1111_1110,
                &(TestExternal::from_bits_retain(!0) & !TestExternal::A).bits(),
            ) {
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
            match (
                &0b1111_1110,
                &(TestFlags::from_bits_retain(!0).difference(TestFlags::A)).bits(),
            ) {
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
            match (
                &(1 << 1 | 1 << 2),
                &(TestFlags::from_bits_retain(!0) & !TestFlags::A).bits(),
            ) {
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
        #[track_caller]
        fn case<
            T: Flags + std::fmt::Debug + std::ops::Sub<Output = T> + std::ops::SubAssign
                + Copy,
        >(value: T, inputs: &[(T, T::Bits)], mut inherent: impl FnMut(T, T) -> T)
        where
            T::Bits: std::fmt::Debug + PartialEq + Copy,
        {
            for (input, expected) in inputs {
                match (&*expected, &inherent(value, *input).bits()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("{0:?}.difference({1:?})", value, input),
                                ),
                            );
                        }
                    }
                };
                match (&*expected, &Flags::difference(value, *input).bits()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!(
                                        "Flags::difference({0:?}, {1:?})",
                                        value,
                                        input,
                                    ),
                                ),
                            );
                        }
                    }
                };
                match (&*expected, &(value - *input).bits()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("{0:?} - {1:?}", value, input),
                                ),
                            );
                        }
                    }
                };
                match (
                    &*expected,
                    &{
                        let mut value = value;
                        value -= *input;
                        value
                    }
                        .bits(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("{0:?} -= {1:?}", value, input),
                                ),
                            );
                        }
                    }
                };
            }
        }
    }
    mod empty {
        use super::*;
        use crate::Flags;
        extern crate test;
        #[rustc_test_marker = "tests::empty::cases"]
        #[doc(hidden)]
        pub const cases: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::empty::cases"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/empty.rs",
                start_line: 6usize,
                start_col: 4usize,
                end_line: 6usize,
                end_col: 9usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(cases()),
            ),
        };
        fn cases() {
            case(0, TestFlags::empty);
            case(0, TestZero::empty);
            case(0, TestEmpty::empty);
            case(0, TestExternal::empty);
        }
        #[track_caller]
        fn case<T: Flags>(expected: T::Bits, inherent: impl FnOnce() -> T)
        where
            <T as Flags>::Bits: std::fmt::Debug + PartialEq,
        {
            match (&expected, &inherent().bits()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(format_args!("T::empty()")),
                        );
                    }
                }
            };
            match (&expected, &T::empty().bits()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(format_args!("Flags::empty()")),
                        );
                    }
                }
            };
        }
    }
    mod eq {
        use super::*;
        extern crate test;
        #[rustc_test_marker = "tests::eq::cases"]
        #[doc(hidden)]
        pub const cases: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::eq::cases"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/eq.rs",
                start_line: 4usize,
                start_col: 4usize,
                end_line: 4usize,
                end_col: 9usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(cases()),
            ),
        };
        fn cases() {
            match (&TestFlags::empty(), &TestFlags::empty()) {
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
            match (&TestFlags::all(), &TestFlags::all()) {
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
            if !(TestFlags::from_bits_retain(1) < TestFlags::from_bits_retain(2)) {
                ::core::panicking::panic(
                    "assertion failed: TestFlags::from_bits_retain(1) < TestFlags::from_bits_retain(2)",
                )
            }
            if !(TestFlags::from_bits_retain(2) > TestFlags::from_bits_retain(1)) {
                ::core::panicking::panic(
                    "assertion failed: TestFlags::from_bits_retain(2) > TestFlags::from_bits_retain(1)",
                )
            }
        }
    }
    mod extend {
        use super::*;
        extern crate test;
        #[rustc_test_marker = "tests::extend::cases"]
        #[doc(hidden)]
        pub const cases: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::extend::cases"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/extend.rs",
                start_line: 4usize,
                start_col: 4usize,
                end_line: 4usize,
                end_col: 9usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(cases()),
            ),
        };
        fn cases() {
            let mut flags = TestFlags::empty();
            flags.extend(TestFlags::A);
            match (&TestFlags::A, &flags) {
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
            flags.extend(TestFlags::A | TestFlags::B | TestFlags::C);
            match (&TestFlags::ABC, &flags) {
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
            flags.extend(TestFlags::from_bits_retain(1 << 5));
            match (&(TestFlags::ABC | TestFlags::from_bits_retain(1 << 5)), &flags) {
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
        mod external {
            use super::*;
            extern crate test;
            #[rustc_test_marker = "tests::extend::external::cases"]
            #[doc(hidden)]
            pub const cases: test::TestDescAndFn = test::TestDescAndFn {
                desc: test::TestDesc {
                    name: test::StaticTestName("tests::extend::external::cases"),
                    ignore: false,
                    ignore_message: ::core::option::Option::None,
                    source_file: "src/tests/extend.rs",
                    start_line: 24usize,
                    start_col: 8usize,
                    end_line: 24usize,
                    end_col: 13usize,
                    compile_fail: false,
                    no_run: false,
                    should_panic: test::ShouldPanic::No,
                    test_type: test::TestType::UnitTest,
                },
                testfn: test::StaticTestFn(
                    #[coverage(off)]
                    || test::assert_test_result(cases()),
                ),
            };
            fn cases() {
                let mut flags = TestExternal::empty();
                flags.extend(TestExternal::A);
                match (&TestExternal::A, &flags) {
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
                flags.extend(TestExternal::A | TestExternal::B | TestExternal::C);
                match (&TestExternal::ABC, &flags) {
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
                flags.extend(TestExternal::from_bits_retain(1 << 5));
                match (
                    &(TestExternal::ABC | TestExternal::from_bits_retain(1 << 5)),
                    &flags,
                ) {
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
    mod flags {
        use super::*;
        use crate::Flags;
        extern crate test;
        #[rustc_test_marker = "tests::flags::cases"]
        #[doc(hidden)]
        pub const cases: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::flags::cases"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/flags.rs",
                start_line: 6usize,
                start_col: 4usize,
                end_line: 6usize,
                end_col: 9usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(cases()),
            ),
        };
        fn cases() {
            let flags = TestFlags::FLAGS
                .iter()
                .map(|flag| (flag.name(), flag.value().bits()))
                .collect::<Vec<_>>();
            match (
                &<[_]>::into_vec(
                    ::alloc::boxed::box_new([
                        ("A", 1u8),
                        ("B", 1 << 1),
                        ("C", 1 << 2),
                        ("ABC", 1 | 1 << 1 | 1 << 2),
                    ]),
                ),
                &flags,
            ) {
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
            match (&0, &TestEmpty::FLAGS.iter().count()) {
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
        mod external {
            use super::*;
            extern crate test;
            #[rustc_test_marker = "tests::flags::external::cases"]
            #[doc(hidden)]
            pub const cases: test::TestDescAndFn = test::TestDescAndFn {
                desc: test::TestDesc {
                    name: test::StaticTestName("tests::flags::external::cases"),
                    ignore: false,
                    ignore_message: ::core::option::Option::None,
                    source_file: "src/tests/flags.rs",
                    start_line: 29usize,
                    start_col: 8usize,
                    end_line: 29usize,
                    end_col: 13usize,
                    compile_fail: false,
                    no_run: false,
                    should_panic: test::ShouldPanic::No,
                    test_type: test::TestType::UnitTest,
                },
                testfn: test::StaticTestFn(
                    #[coverage(off)]
                    || test::assert_test_result(cases()),
                ),
            };
            fn cases() {
                let flags = TestExternal::FLAGS
                    .iter()
                    .map(|flag| (flag.name(), flag.value().bits()))
                    .collect::<Vec<_>>();
                match (
                    &<[_]>::into_vec(
                        ::alloc::boxed::box_new([
                            ("A", 1u8),
                            ("B", 1 << 1),
                            ("C", 1 << 2),
                            ("ABC", 1 | 1 << 1 | 1 << 2),
                            ("", !0),
                        ]),
                    ),
                    &flags,
                ) {
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
    mod fmt {
        use super::*;
        extern crate test;
        #[rustc_test_marker = "tests::fmt::cases"]
        #[doc(hidden)]
        pub const cases: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::fmt::cases"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/fmt.rs",
                start_line: 4usize,
                start_col: 4usize,
                end_line: 4usize,
                end_col: 9usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(cases()),
            ),
        };
        fn cases() {
            case(TestFlags::empty(), "TestFlags(0x0)", "0", "0", "0", "0");
            case(TestFlags::A, "TestFlags(A)", "1", "1", "1", "1");
            case(TestFlags::all(), "TestFlags(A | B | C)", "7", "7", "7", "111");
            case(
                TestFlags::from_bits_retain(1 << 3),
                "TestFlags(0x8)",
                "8",
                "8",
                "10",
                "1000",
            );
            case(
                TestFlags::A | TestFlags::from_bits_retain(1 << 3),
                "TestFlags(A | 0x8)",
                "9",
                "9",
                "11",
                "1001",
            );
            case(TestZero::ZERO, "TestZero(0x0)", "0", "0", "0", "0");
            case(
                TestZero::ZERO | TestZero::from_bits_retain(1),
                "TestZero(0x1)",
                "1",
                "1",
                "1",
                "1",
            );
            case(TestZeroOne::ONE, "TestZeroOne(ONE)", "1", "1", "1", "1");
            case(
                TestOverlapping::from_bits_retain(1 << 1),
                "TestOverlapping(0x2)",
                "2",
                "2",
                "2",
                "10",
            );
            case(
                TestExternal::from_bits_retain(1 | 1 << 1 | 1 << 3),
                "TestExternal(A | B | 0x8)",
                "B",
                "b",
                "13",
                "1011",
            );
            case(
                TestExternal::all(),
                "TestExternal(A | B | C | 0xf8)",
                "FF",
                "ff",
                "377",
                "11111111",
            );
            case(
                TestExternalFull::all(),
                "TestExternalFull(0xff)",
                "FF",
                "ff",
                "377",
                "11111111",
            );
        }
        #[track_caller]
        fn case<
            T: std::fmt::Debug + std::fmt::UpperHex + std::fmt::LowerHex
                + std::fmt::Octal + std::fmt::Binary,
        >(value: T, debug: &str, uhex: &str, lhex: &str, oct: &str, bin: &str) {
            match (
                &debug,
                &::alloc::__export::must_use({
                    ::alloc::fmt::format(format_args!("{0:?}", value))
                }),
            ) {
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
            match (
                &uhex,
                &::alloc::__export::must_use({
                    ::alloc::fmt::format(format_args!("{0:X}", value))
                }),
            ) {
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
            match (
                &lhex,
                &::alloc::__export::must_use({
                    ::alloc::fmt::format(format_args!("{0:x}", value))
                }),
            ) {
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
            match (
                &oct,
                &::alloc::__export::must_use({
                    ::alloc::fmt::format(format_args!("{0:o}", value))
                }),
            ) {
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
            match (
                &bin,
                &::alloc::__export::must_use({
                    ::alloc::fmt::format(format_args!("{0:b}", value))
                }),
            ) {
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
    mod from_bits {
        use super::*;
        use crate::Flags;
        extern crate test;
        #[rustc_test_marker = "tests::from_bits::cases"]
        #[doc(hidden)]
        pub const cases: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::from_bits::cases"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/from_bits.rs",
                start_line: 6usize,
                start_col: 4usize,
                end_line: 6usize,
                end_col: 9usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(cases()),
            ),
        };
        fn cases() {
            case(Some(0), 0, TestFlags::from_bits);
            case(Some(1), 1, TestFlags::from_bits);
            case(Some(1 | 1 << 1 | 1 << 2), 1 | 1 << 1 | 1 << 2, TestFlags::from_bits);
            case(None, 1 << 3, TestFlags::from_bits);
            case(None, 1 | 1 << 3, TestFlags::from_bits);
            case(Some(1 | 1 << 1), 1 | 1 << 1, TestOverlapping::from_bits);
            case(Some(1 << 1), 1 << 1, TestOverlapping::from_bits);
            case(Some(1 << 5), 1 << 5, TestExternal::from_bits);
        }
        #[track_caller]
        fn case<T: Flags>(
            expected: Option<T::Bits>,
            input: T::Bits,
            inherent: impl FnOnce(T::Bits) -> Option<T>,
        )
        where
            <T as Flags>::Bits: std::fmt::Debug + PartialEq,
        {
            match (&expected, &inherent(input).map(|f| f.bits())) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("T::from_bits({0:?})", input),
                            ),
                        );
                    }
                }
            };
            match (&expected, &T::from_bits(input).map(|f| f.bits())) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("Flags::from_bits({0:?})", input),
                            ),
                        );
                    }
                }
            };
        }
    }
    mod from_bits_retain {
        use super::*;
        use crate::Flags;
        extern crate test;
        #[rustc_test_marker = "tests::from_bits_retain::cases"]
        #[doc(hidden)]
        pub const cases: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::from_bits_retain::cases"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/from_bits_retain.rs",
                start_line: 6usize,
                start_col: 4usize,
                end_line: 6usize,
                end_col: 9usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(cases()),
            ),
        };
        fn cases() {
            case(0, TestFlags::from_bits_retain);
            case(1, TestFlags::from_bits_retain);
            case(1 | 1 << 1 | 1 << 2, TestFlags::from_bits_retain);
            case(1 << 3, TestFlags::from_bits_retain);
            case(1 | 1 << 3, TestFlags::from_bits_retain);
            case(1 | 1 << 1, TestOverlapping::from_bits_retain);
            case(1 << 1, TestOverlapping::from_bits_retain);
            case(1 << 5, TestExternal::from_bits_retain);
        }
        #[track_caller]
        fn case<T: Flags>(input: T::Bits, inherent: impl FnOnce(T::Bits) -> T)
        where
            <T as Flags>::Bits: std::fmt::Debug + PartialEq,
        {
            match (&input, &inherent(input).bits()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("T::from_bits_retain({0:?})", input),
                            ),
                        );
                    }
                }
            };
            match (&input, &T::from_bits_retain(input).bits()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("Flags::from_bits_retain({0:?})", input),
                            ),
                        );
                    }
                }
            };
        }
    }
    mod from_bits_truncate {
        use super::*;
        use crate::Flags;
        extern crate test;
        #[rustc_test_marker = "tests::from_bits_truncate::cases"]
        #[doc(hidden)]
        pub const cases: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::from_bits_truncate::cases"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/from_bits_truncate.rs",
                start_line: 6usize,
                start_col: 4usize,
                end_line: 6usize,
                end_col: 9usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(cases()),
            ),
        };
        fn cases() {
            case(0, 0, TestFlags::from_bits_truncate);
            case(1, 1, TestFlags::from_bits_truncate);
            case(
                1 | 1 << 1 | 1 << 2,
                1 | 1 << 1 | 1 << 2,
                TestFlags::from_bits_truncate,
            );
            case(0, 1 << 3, TestFlags::from_bits_truncate);
            case(1, 1 | 1 << 3, TestFlags::from_bits_truncate);
            case(1 | 1 << 1, 1 | 1 << 1, TestOverlapping::from_bits_truncate);
            case(1 << 1, 1 << 1, TestOverlapping::from_bits_truncate);
            case(1 << 5, 1 << 5, TestExternal::from_bits_truncate);
        }
        #[track_caller]
        fn case<T: Flags>(
            expected: T::Bits,
            input: T::Bits,
            inherent: impl FnOnce(T::Bits) -> T,
        )
        where
            <T as Flags>::Bits: std::fmt::Debug + PartialEq,
        {
            match (&expected, &inherent(input).bits()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("T::from_bits_truncate({0:?})", input),
                            ),
                        );
                    }
                }
            };
            match (&expected, &T::from_bits_truncate(input).bits()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("Flags::from_bits_truncate({0:?})", input),
                            ),
                        );
                    }
                }
            };
        }
    }
    mod from_name {
        use super::*;
        use crate::Flags;
        extern crate test;
        #[rustc_test_marker = "tests::from_name::cases"]
        #[doc(hidden)]
        pub const cases: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::from_name::cases"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/from_name.rs",
                start_line: 6usize,
                start_col: 4usize,
                end_line: 6usize,
                end_col: 9usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(cases()),
            ),
        };
        fn cases() {
            case(Some(1), "A", TestFlags::from_name);
            case(Some(1 << 1), "B", TestFlags::from_name);
            case(Some(1 | 1 << 1 | 1 << 2), "ABC", TestFlags::from_name);
            case(None, "", TestFlags::from_name);
            case(None, "a", TestFlags::from_name);
            case(None, "0x1", TestFlags::from_name);
            case(None, "A | B", TestFlags::from_name);
            case(Some(0), "ZERO", TestZero::from_name);
            case(Some(2), "二", TestUnicode::from_name);
            case(None, "_", TestExternal::from_name);
            case(None, "", TestExternal::from_name);
        }
        #[track_caller]
        fn case<T: Flags>(
            expected: Option<T::Bits>,
            input: &str,
            inherent: impl FnOnce(&str) -> Option<T>,
        )
        where
            <T as Flags>::Bits: std::fmt::Debug + PartialEq,
        {
            match (&expected, &inherent(input).map(|f| f.bits())) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("T::from_name({0:?})", input),
                            ),
                        );
                    }
                }
            };
            match (&expected, &T::from_name(input).map(|f| f.bits())) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("Flags::from_name({0:?})", input),
                            ),
                        );
                    }
                }
            };
        }
    }
    mod insert {
        use super::*;
        use crate::Flags;
        extern crate test;
        #[rustc_test_marker = "tests::insert::cases"]
        #[doc(hidden)]
        pub const cases: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::insert::cases"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/insert.rs",
                start_line: 6usize,
                start_col: 4usize,
                end_line: 6usize,
                end_col: 9usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(cases()),
            ),
        };
        fn cases() {
            case(
                TestFlags::empty(),
                &[
                    (TestFlags::A, 1),
                    (TestFlags::A | TestFlags::B, 1 | 1 << 1),
                    (TestFlags::empty(), 0),
                    (TestFlags::from_bits_retain(1 << 3), 1 << 3),
                ],
                TestFlags::insert,
                TestFlags::set,
            );
            case(
                TestFlags::A,
                &[
                    (TestFlags::A, 1),
                    (TestFlags::empty(), 1),
                    (TestFlags::B, 1 | 1 << 1),
                ],
                TestFlags::insert,
                TestFlags::set,
            );
        }
        #[track_caller]
        fn case<T: Flags + std::fmt::Debug + Copy>(
            value: T,
            inputs: &[(T, T::Bits)],
            mut inherent_insert: impl FnMut(&mut T, T),
            mut inherent_set: impl FnMut(&mut T, T, bool),
        )
        where
            T::Bits: std::fmt::Debug + PartialEq + Copy,
        {
            for (input, expected) in inputs {
                match (
                    &*expected,
                    &{
                        let mut value = value;
                        inherent_insert(&mut value, *input);
                        value
                    }
                        .bits(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("{0:?}.insert({1:?})", value, input),
                                ),
                            );
                        }
                    }
                };
                match (
                    &*expected,
                    &{
                        let mut value = value;
                        Flags::insert(&mut value, *input);
                        value
                    }
                        .bits(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("Flags::insert({0:?}, {1:?})", value, input),
                                ),
                            );
                        }
                    }
                };
                match (
                    &*expected,
                    &{
                        let mut value = value;
                        inherent_set(&mut value, *input, true);
                        value
                    }
                        .bits(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("{0:?}.set({1:?}, true)", value, input),
                                ),
                            );
                        }
                    }
                };
                match (
                    &*expected,
                    &{
                        let mut value = value;
                        Flags::set(&mut value, *input, true);
                        value
                    }
                        .bits(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("Flags::set({0:?}, {1:?}, true)", value, input),
                                ),
                            );
                        }
                    }
                };
            }
        }
    }
    mod intersection {
        use super::*;
        use crate::Flags;
        extern crate test;
        #[rustc_test_marker = "tests::intersection::cases"]
        #[doc(hidden)]
        pub const cases: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::intersection::cases"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/intersection.rs",
                start_line: 6usize,
                start_col: 4usize,
                end_line: 6usize,
                end_col: 9usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(cases()),
            ),
        };
        fn cases() {
            case(
                TestFlags::empty(),
                &[(TestFlags::empty(), 0), (TestFlags::all(), 0)],
                TestFlags::intersection,
            );
            case(
                TestFlags::all(),
                &[
                    (TestFlags::all(), 1 | 1 << 1 | 1 << 2),
                    (TestFlags::A, 1),
                    (TestFlags::from_bits_retain(1 << 3), 0),
                ],
                TestFlags::intersection,
            );
            case(
                TestFlags::from_bits_retain(1 << 3),
                &[(TestFlags::from_bits_retain(1 << 3), 1 << 3)],
                TestFlags::intersection,
            );
            case(
                TestOverlapping::AB,
                &[(TestOverlapping::BC, 1 << 1)],
                TestOverlapping::intersection,
            );
        }
        #[track_caller]
        fn case<
            T: Flags + std::fmt::Debug + std::ops::BitAnd<Output = T>
                + std::ops::BitAndAssign + Copy,
        >(value: T, inputs: &[(T, T::Bits)], mut inherent: impl FnMut(T, T) -> T)
        where
            T::Bits: std::fmt::Debug + PartialEq + Copy,
        {
            for (input, expected) in inputs {
                match (&*expected, &inherent(value, *input).bits()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("{0:?}.intersection({1:?})", value, input),
                                ),
                            );
                        }
                    }
                };
                match (&*expected, &Flags::intersection(value, *input).bits()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!(
                                        "Flags::intersection({0:?}, {1:?})",
                                        value,
                                        input,
                                    ),
                                ),
                            );
                        }
                    }
                };
                match (&*expected, &(value & *input).bits()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("{0:?} & {1:?}", value, input),
                                ),
                            );
                        }
                    }
                };
                match (
                    &*expected,
                    &{
                        let mut value = value;
                        value &= *input;
                        value
                    }
                        .bits(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("{0:?} &= {1:?}", value, input),
                                ),
                            );
                        }
                    }
                };
            }
        }
    }
    mod intersects {
        use super::*;
        use crate::Flags;
        extern crate test;
        #[rustc_test_marker = "tests::intersects::cases"]
        #[doc(hidden)]
        pub const cases: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::intersects::cases"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/intersects.rs",
                start_line: 6usize,
                start_col: 4usize,
                end_line: 6usize,
                end_col: 9usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(cases()),
            ),
        };
        fn cases() {
            case(
                TestFlags::empty(),
                &[
                    (TestFlags::empty(), false),
                    (TestFlags::A, false),
                    (TestFlags::B, false),
                    (TestFlags::C, false),
                    (TestFlags::from_bits_retain(1 << 3), false),
                ],
                TestFlags::intersects,
            );
            case(
                TestFlags::A,
                &[
                    (TestFlags::empty(), false),
                    (TestFlags::A, true),
                    (TestFlags::B, false),
                    (TestFlags::C, false),
                    (TestFlags::ABC, true),
                    (TestFlags::from_bits_retain(1 << 3), false),
                    (TestFlags::from_bits_retain(1 | (1 << 3)), true),
                ],
                TestFlags::intersects,
            );
            case(
                TestFlags::ABC,
                &[
                    (TestFlags::empty(), false),
                    (TestFlags::A, true),
                    (TestFlags::B, true),
                    (TestFlags::C, true),
                    (TestFlags::ABC, true),
                    (TestFlags::from_bits_retain(1 << 3), false),
                ],
                TestFlags::intersects,
            );
            case(
                TestFlags::from_bits_retain(1 << 3),
                &[
                    (TestFlags::empty(), false),
                    (TestFlags::A, false),
                    (TestFlags::B, false),
                    (TestFlags::C, false),
                    (TestFlags::from_bits_retain(1 << 3), true),
                ],
                TestFlags::intersects,
            );
            case(
                TestOverlapping::AB,
                &[
                    (TestOverlapping::AB, true),
                    (TestOverlapping::BC, true),
                    (TestOverlapping::from_bits_retain(1 << 1), true),
                ],
                TestOverlapping::intersects,
            );
        }
        #[track_caller]
        fn case<T: Flags + std::fmt::Debug + Copy>(
            value: T,
            inputs: &[(T, bool)],
            mut inherent: impl FnMut(&T, T) -> bool,
        ) {
            for (input, expected) in inputs {
                match (&*expected, &inherent(&value, *input)) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("{0:?}.intersects({1:?})", value, input),
                                ),
                            );
                        }
                    }
                };
                match (&*expected, &Flags::intersects(&value, *input)) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!(
                                        "Flags::intersects({0:?}, {1:?})",
                                        value,
                                        input,
                                    ),
                                ),
                            );
                        }
                    }
                };
            }
        }
    }
    mod is_all {
        use super::*;
        use crate::Flags;
        extern crate test;
        #[rustc_test_marker = "tests::is_all::cases"]
        #[doc(hidden)]
        pub const cases: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::is_all::cases"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/is_all.rs",
                start_line: 6usize,
                start_col: 4usize,
                end_line: 6usize,
                end_col: 9usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(cases()),
            ),
        };
        fn cases() {
            case(false, TestFlags::empty(), TestFlags::is_all);
            case(false, TestFlags::A, TestFlags::is_all);
            case(true, TestFlags::ABC, TestFlags::is_all);
            case(
                true,
                TestFlags::ABC | TestFlags::from_bits_retain(1 << 3),
                TestFlags::is_all,
            );
            case(true, TestZero::empty(), TestZero::is_all);
            case(true, TestEmpty::empty(), TestEmpty::is_all);
        }
        #[track_caller]
        fn case<T: Flags + std::fmt::Debug>(
            expected: bool,
            value: T,
            inherent: impl FnOnce(&T) -> bool,
        ) {
            match (&expected, &inherent(&value)) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("{0:?}.is_all()", value),
                            ),
                        );
                    }
                }
            };
            match (&expected, &Flags::is_all(&value)) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("Flags::is_all({0:?})", value),
                            ),
                        );
                    }
                }
            };
        }
    }
    mod is_empty {
        use super::*;
        use crate::Flags;
        extern crate test;
        #[rustc_test_marker = "tests::is_empty::cases"]
        #[doc(hidden)]
        pub const cases: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::is_empty::cases"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/is_empty.rs",
                start_line: 6usize,
                start_col: 4usize,
                end_line: 6usize,
                end_col: 9usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(cases()),
            ),
        };
        fn cases() {
            case(true, TestFlags::empty(), TestFlags::is_empty);
            case(false, TestFlags::A, TestFlags::is_empty);
            case(false, TestFlags::ABC, TestFlags::is_empty);
            case(false, TestFlags::from_bits_retain(1 << 3), TestFlags::is_empty);
            case(true, TestZero::empty(), TestZero::is_empty);
            case(true, TestEmpty::empty(), TestEmpty::is_empty);
        }
        #[track_caller]
        fn case<T: Flags + std::fmt::Debug>(
            expected: bool,
            value: T,
            inherent: impl FnOnce(&T) -> bool,
        ) {
            match (&expected, &inherent(&value)) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("{0:?}.is_empty()", value),
                            ),
                        );
                    }
                }
            };
            match (&expected, &Flags::is_empty(&value)) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("Flags::is_empty({0:?})", value),
                            ),
                        );
                    }
                }
            };
        }
    }
    mod iter {
        use super::*;
        use crate::Flags;
        extern crate test;
        #[rustc_test_marker = "tests::iter::roundtrip"]
        #[doc(hidden)]
        pub const roundtrip: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::iter::roundtrip"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/iter.rs",
                start_line: 7usize,
                start_col: 4usize,
                end_line: 7usize,
                end_col: 13usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(roundtrip()),
            ),
        };
        fn roundtrip() {
            for a in 0u8..=255 {
                for b in 0u8..=255 {
                    let f = TestFlags::from_bits_retain(a | b);
                    match (&f, &f.iter().collect::<TestFlags>()) {
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
                    match (
                        &TestFlags::from_bits_truncate(f.bits()),
                        &f.iter_names().map(|(_, f)| f).collect::<TestFlags>(),
                    ) {
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
                    let f = TestExternal::from_bits_retain(a | b);
                    match (&f, &f.iter().collect::<TestExternal>()) {
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
        mod collect {
            use super::*;
            extern crate test;
            #[rustc_test_marker = "tests::iter::collect::cases"]
            #[doc(hidden)]
            pub const cases: test::TestDescAndFn = test::TestDescAndFn {
                desc: test::TestDesc {
                    name: test::StaticTestName("tests::iter::collect::cases"),
                    ignore: false,
                    ignore_message: ::core::option::Option::None,
                    source_file: "src/tests/iter.rs",
                    start_line: 29usize,
                    start_col: 8usize,
                    end_line: 29usize,
                    end_col: 13usize,
                    compile_fail: false,
                    no_run: false,
                    should_panic: test::ShouldPanic::No,
                    test_type: test::TestType::UnitTest,
                },
                testfn: test::StaticTestFn(
                    #[coverage(off)]
                    || test::assert_test_result(cases()),
                ),
            };
            fn cases() {
                match (&0, &[].into_iter().collect::<TestFlags>().bits()) {
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
                match (&1, &[TestFlags::A].into_iter().collect::<TestFlags>().bits()) {
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
                match (
                    &(1 | 1 << 1 | 1 << 2),
                    &[TestFlags::A, TestFlags::B | TestFlags::C]
                        .into_iter()
                        .collect::<TestFlags>()
                        .bits(),
                ) {
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
                match (
                    &(1 | 1 << 3),
                    &[
                        TestFlags::from_bits_retain(1 << 3),
                        TestFlags::empty(),
                        TestFlags::A,
                    ]
                        .into_iter()
                        .collect::<TestFlags>()
                        .bits(),
                ) {
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
                match (
                    &(1 << 5 | 1 << 7),
                    &[
                        TestExternal::empty(),
                        TestExternal::from_bits_retain(1 << 5),
                        TestExternal::from_bits_retain(1 << 7),
                    ]
                        .into_iter()
                        .collect::<TestExternal>()
                        .bits(),
                ) {
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
        mod iter {
            use super::*;
            extern crate test;
            #[rustc_test_marker = "tests::iter::iter::cases"]
            #[doc(hidden)]
            pub const cases: test::TestDescAndFn = test::TestDescAndFn {
                desc: test::TestDesc {
                    name: test::StaticTestName("tests::iter::iter::cases"),
                    ignore: false,
                    ignore_message: ::core::option::Option::None,
                    source_file: "src/tests/iter.rs",
                    start_line: 72usize,
                    start_col: 8usize,
                    end_line: 72usize,
                    end_col: 13usize,
                    compile_fail: false,
                    no_run: false,
                    should_panic: test::ShouldPanic::No,
                    test_type: test::TestType::UnitTest,
                },
                testfn: test::StaticTestFn(
                    #[coverage(off)]
                    || test::assert_test_result(cases()),
                ),
            };
            fn cases() {
                case(&[], TestFlags::empty(), TestFlags::iter);
                case(&[1], TestFlags::A, TestFlags::iter);
                case(&[1, 1 << 1], TestFlags::A | TestFlags::B, TestFlags::iter);
                case(
                    &[1, 1 << 1, 1 << 3],
                    TestFlags::A | TestFlags::B | TestFlags::from_bits_retain(1 << 3),
                    TestFlags::iter,
                );
                case(&[1, 1 << 1, 1 << 2], TestFlags::ABC, TestFlags::iter);
                case(
                    &[1, 1 << 1, 1 << 2, 1 << 3],
                    TestFlags::ABC | TestFlags::from_bits_retain(1 << 3),
                    TestFlags::iter,
                );
                case(
                    &[1 | 1 << 1 | 1 << 2],
                    TestFlagsInvert::ABC,
                    TestFlagsInvert::iter,
                );
                case(&[], TestZero::ZERO, TestZero::iter);
                case(
                    &[1, 1 << 1, 1 << 2, 0b1111_1000],
                    TestExternal::all(),
                    TestExternal::iter,
                );
            }
            #[track_caller]
            fn case<T: Flags + std::fmt::Debug + IntoIterator<Item = T> + Copy>(
                expected: &[T::Bits],
                value: T,
                inherent: impl FnOnce(&T) -> crate::iter::Iter<T>,
            )
            where
                T::Bits: std::fmt::Debug + PartialEq,
            {
                match (
                    &expected,
                    &inherent(&value).map(|f| f.bits()).collect::<Vec<_>>(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("{0:?}.iter()", value),
                                ),
                            );
                        }
                    }
                };
                match (
                    &expected,
                    &Flags::iter(&value).map(|f| f.bits()).collect::<Vec<_>>(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("Flags::iter({0:?})", value),
                                ),
                            );
                        }
                    }
                };
                match (
                    &expected,
                    &value.into_iter().map(|f| f.bits()).collect::<Vec<_>>(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("{0:?}.into_iter()", value),
                                ),
                            );
                        }
                    }
                };
            }
        }
        mod iter_names {
            use super::*;
            extern crate test;
            #[rustc_test_marker = "tests::iter::iter_names::cases"]
            #[doc(hidden)]
            pub const cases: test::TestDescAndFn = test::TestDescAndFn {
                desc: test::TestDesc {
                    name: test::StaticTestName("tests::iter::iter_names::cases"),
                    ignore: false,
                    ignore_message: ::core::option::Option::None,
                    source_file: "src/tests/iter.rs",
                    start_line: 138usize,
                    start_col: 8usize,
                    end_line: 138usize,
                    end_col: 13usize,
                    compile_fail: false,
                    no_run: false,
                    should_panic: test::ShouldPanic::No,
                    test_type: test::TestType::UnitTest,
                },
                testfn: test::StaticTestFn(
                    #[coverage(off)]
                    || test::assert_test_result(cases()),
                ),
            };
            fn cases() {
                case(&[], TestFlags::empty(), TestFlags::iter_names);
                case(&[("A", 1)], TestFlags::A, TestFlags::iter_names);
                case(
                    &[("A", 1), ("B", 1 << 1)],
                    TestFlags::A | TestFlags::B,
                    TestFlags::iter_names,
                );
                case(
                    &[("A", 1), ("B", 1 << 1)],
                    TestFlags::A | TestFlags::B | TestFlags::from_bits_retain(1 << 3),
                    TestFlags::iter_names,
                );
                case(
                    &[("A", 1), ("B", 1 << 1), ("C", 1 << 2)],
                    TestFlags::ABC,
                    TestFlags::iter_names,
                );
                case(
                    &[("A", 1), ("B", 1 << 1), ("C", 1 << 2)],
                    TestFlags::ABC | TestFlags::from_bits_retain(1 << 3),
                    TestFlags::iter_names,
                );
                case(
                    &[("ABC", 1 | 1 << 1 | 1 << 2)],
                    TestFlagsInvert::ABC,
                    TestFlagsInvert::iter_names,
                );
                case(&[], TestZero::ZERO, TestZero::iter_names);
                case(
                    &[("A", 1)],
                    TestOverlappingFull::A,
                    TestOverlappingFull::iter_names,
                );
                case(
                    &[("A", 1), ("D", 1 << 1)],
                    TestOverlappingFull::A | TestOverlappingFull::D,
                    TestOverlappingFull::iter_names,
                );
            }
            #[track_caller]
            fn case<T: Flags + std::fmt::Debug>(
                expected: &[(&'static str, T::Bits)],
                value: T,
                inherent: impl FnOnce(&T) -> crate::iter::IterNames<T>,
            )
            where
                T::Bits: std::fmt::Debug + PartialEq,
            {
                match (
                    &expected,
                    &inherent(&value).map(|(n, f)| (n, f.bits())).collect::<Vec<_>>(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("{0:?}.iter_names()", value),
                                ),
                            );
                        }
                    }
                };
                match (
                    &expected,
                    &Flags::iter_names(&value)
                        .map(|(n, f)| (n, f.bits()))
                        .collect::<Vec<_>>(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("Flags::iter_names({0:?})", value),
                                ),
                            );
                        }
                    }
                };
            }
        }
    }
    mod parser {
        use super::*;
        use crate::{parser::*, Flags};
        extern crate test;
        #[rustc_test_marker = "tests::parser::roundtrip"]
        #[doc(hidden)]
        pub const roundtrip: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::parser::roundtrip"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/parser.rs",
                start_line: 7usize,
                start_col: 4usize,
                end_line: 7usize,
                end_col: 13usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(roundtrip()),
            ),
        };
        fn roundtrip() {
            let mut s = String::new();
            for a in 0u8..=255 {
                for b in 0u8..=255 {
                    let f = TestFlags::from_bits_retain(a | b);
                    s.clear();
                    to_writer(&f, &mut s).unwrap();
                    match (&f, &from_str::<TestFlags>(&s).unwrap()) {
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
        extern crate test;
        #[rustc_test_marker = "tests::parser::roundtrip_truncate"]
        #[doc(hidden)]
        pub const roundtrip_truncate: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::parser::roundtrip_truncate"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/parser.rs",
                start_line: 24usize,
                start_col: 4usize,
                end_line: 24usize,
                end_col: 22usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(roundtrip_truncate()),
            ),
        };
        fn roundtrip_truncate() {
            let mut s = String::new();
            for a in 0u8..=255 {
                for b in 0u8..=255 {
                    let f = TestFlags::from_bits_retain(a | b);
                    s.clear();
                    to_writer_truncate(&f, &mut s).unwrap();
                    match (
                        &TestFlags::from_bits_truncate(f.bits()),
                        &from_str_truncate::<TestFlags>(&s).unwrap(),
                    ) {
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
        extern crate test;
        #[rustc_test_marker = "tests::parser::roundtrip_strict"]
        #[doc(hidden)]
        pub const roundtrip_strict: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::parser::roundtrip_strict"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/parser.rs",
                start_line: 44usize,
                start_col: 4usize,
                end_line: 44usize,
                end_col: 20usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(roundtrip_strict()),
            ),
        };
        fn roundtrip_strict() {
            let mut s = String::new();
            for a in 0u8..=255 {
                for b in 0u8..=255 {
                    let f = TestFlags::from_bits_retain(a | b);
                    s.clear();
                    to_writer_strict(&f, &mut s).unwrap();
                    let mut strict = TestFlags::empty();
                    for (_, flag) in f.iter_names() {
                        strict |= flag;
                    }
                    let f = strict;
                    if let Ok(s) = from_str_strict::<TestFlags>(&s) {
                        match (&f, &s) {
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
        mod from_str {
            use super::*;
            extern crate test;
            #[rustc_test_marker = "tests::parser::from_str::valid"]
            #[doc(hidden)]
            pub const valid: test::TestDescAndFn = test::TestDescAndFn {
                desc: test::TestDesc {
                    name: test::StaticTestName("tests::parser::from_str::valid"),
                    ignore: false,
                    ignore_message: ::core::option::Option::None,
                    source_file: "src/tests/parser.rs",
                    start_line: 71usize,
                    start_col: 8usize,
                    end_line: 71usize,
                    end_col: 13usize,
                    compile_fail: false,
                    no_run: false,
                    should_panic: test::ShouldPanic::No,
                    test_type: test::TestType::UnitTest,
                },
                testfn: test::StaticTestFn(
                    #[coverage(off)]
                    || test::assert_test_result(valid()),
                ),
            };
            fn valid() {
                match (&0, &from_str::<TestFlags>("").unwrap().bits()) {
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
                match (&1, &from_str::<TestFlags>("A").unwrap().bits()) {
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
                match (&1, &from_str::<TestFlags>(" A ").unwrap().bits()) {
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
                match (
                    &(1 | 1 << 1 | 1 << 2),
                    &from_str::<TestFlags>("A | B | C").unwrap().bits(),
                ) {
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
                match (
                    &(1 | 1 << 1 | 1 << 2),
                    &from_str::<TestFlags>("A\n|\tB\r\n|   C ").unwrap().bits(),
                ) {
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
                match (
                    &(1 | 1 << 1 | 1 << 2),
                    &from_str::<TestFlags>("A|B|C").unwrap().bits(),
                ) {
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
                match (&(1 << 3), &from_str::<TestFlags>("0x8").unwrap().bits()) {
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
                match (
                    &(1 | 1 << 3),
                    &from_str::<TestFlags>("A | 0x8").unwrap().bits(),
                ) {
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
                match (
                    &(1 | 1 << 1 | 1 << 3),
                    &from_str::<TestFlags>("0x1 | 0x8 | B").unwrap().bits(),
                ) {
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
                match (
                    &(1 | 1 << 1),
                    &from_str::<TestUnicode>("一 | 二").unwrap().bits(),
                ) {
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
            #[rustc_test_marker = "tests::parser::from_str::invalid"]
            #[doc(hidden)]
            pub const invalid: test::TestDescAndFn = test::TestDescAndFn {
                desc: test::TestDesc {
                    name: test::StaticTestName("tests::parser::from_str::invalid"),
                    ignore: false,
                    ignore_message: ::core::option::Option::None,
                    source_file: "src/tests/parser.rs",
                    start_line: 103usize,
                    start_col: 8usize,
                    end_line: 103usize,
                    end_col: 15usize,
                    compile_fail: false,
                    no_run: false,
                    should_panic: test::ShouldPanic::No,
                    test_type: test::TestType::UnitTest,
                },
                testfn: test::StaticTestFn(
                    #[coverage(off)]
                    || test::assert_test_result(invalid()),
                ),
            };
            fn invalid() {
                if !from_str::<TestFlags>("a")
                    .unwrap_err()
                    .to_string()
                    .starts_with("unrecognized named flag")
                {
                    ::core::panicking::panic(
                        "assertion failed: from_str::<TestFlags>(\"a\").unwrap_err().to_string().starts_with(\"unrecognized named flag\")",
                    )
                }
                if !from_str::<TestFlags>("A & B")
                    .unwrap_err()
                    .to_string()
                    .starts_with("unrecognized named flag")
                {
                    ::core::panicking::panic(
                        "assertion failed: from_str::<TestFlags>(\"A & B\").unwrap_err().to_string().starts_with(\"unrecognized named flag\")",
                    )
                }
                if !from_str::<TestFlags>("0xg")
                    .unwrap_err()
                    .to_string()
                    .starts_with("invalid hex flag")
                {
                    ::core::panicking::panic(
                        "assertion failed: from_str::<TestFlags>(\"0xg\").unwrap_err().to_string().starts_with(\"invalid hex flag\")",
                    )
                }
                if !from_str::<TestFlags>("0xffffffffffff")
                    .unwrap_err()
                    .to_string()
                    .starts_with("invalid hex flag")
                {
                    ::core::panicking::panic(
                        "assertion failed: from_str::<TestFlags>(\"0xffffffffffff\").unwrap_err().to_string().starts_with(\"invalid hex flag\")",
                    )
                }
            }
        }
        mod to_writer {
            use super::*;
            extern crate test;
            #[rustc_test_marker = "tests::parser::to_writer::cases"]
            #[doc(hidden)]
            pub const cases: test::TestDescAndFn = test::TestDescAndFn {
                desc: test::TestDesc {
                    name: test::StaticTestName("tests::parser::to_writer::cases"),
                    ignore: false,
                    ignore_message: ::core::option::Option::None,
                    source_file: "src/tests/parser.rs",
                    start_line: 128usize,
                    start_col: 8usize,
                    end_line: 128usize,
                    end_col: 13usize,
                    compile_fail: false,
                    no_run: false,
                    should_panic: test::ShouldPanic::No,
                    test_type: test::TestType::UnitTest,
                },
                testfn: test::StaticTestFn(
                    #[coverage(off)]
                    || test::assert_test_result(cases()),
                ),
            };
            fn cases() {
                match (&"", &write(TestFlags::empty())) {
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
                match (&"A", &write(TestFlags::A)) {
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
                match (&"A | B | C", &write(TestFlags::all())) {
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
                match (&"0x8", &write(TestFlags::from_bits_retain(1 << 3))) {
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
                match (
                    &"A | 0x8",
                    &write(TestFlags::A | TestFlags::from_bits_retain(1 << 3)),
                ) {
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
                match (&"", &write(TestZero::ZERO)) {
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
                match (&"ABC", &write(TestFlagsInvert::all())) {
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
                match (&"0x1", &write(TestOverlapping::from_bits_retain(1))) {
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
                match (&"A", &write(TestOverlappingFull::C)) {
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
                match (
                    &"A | D",
                    &write(TestOverlappingFull::C | TestOverlappingFull::D),
                ) {
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
            fn write<F: Flags>(value: F) -> String
            where
                F::Bits: crate::parser::WriteHex,
            {
                let mut s = String::new();
                to_writer(&value, &mut s).unwrap();
                s
            }
        }
        mod from_str_truncate {
            use super::*;
            extern crate test;
            #[rustc_test_marker = "tests::parser::from_str_truncate::valid"]
            #[doc(hidden)]
            pub const valid: test::TestDescAndFn = test::TestDescAndFn {
                desc: test::TestDesc {
                    name: test::StaticTestName(
                        "tests::parser::from_str_truncate::valid",
                    ),
                    ignore: false,
                    ignore_message: ::core::option::Option::None,
                    source_file: "src/tests/parser.rs",
                    start_line: 166usize,
                    start_col: 8usize,
                    end_line: 166usize,
                    end_col: 13usize,
                    compile_fail: false,
                    no_run: false,
                    should_panic: test::ShouldPanic::No,
                    test_type: test::TestType::UnitTest,
                },
                testfn: test::StaticTestFn(
                    #[coverage(off)]
                    || test::assert_test_result(valid()),
                ),
            };
            fn valid() {
                match (&0, &from_str_truncate::<TestFlags>("").unwrap().bits()) {
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
                match (&1, &from_str_truncate::<TestFlags>("A").unwrap().bits()) {
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
                match (&1, &from_str_truncate::<TestFlags>(" A ").unwrap().bits()) {
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
                match (
                    &(1 | 1 << 1 | 1 << 2),
                    &from_str_truncate::<TestFlags>("A | B | C").unwrap().bits(),
                ) {
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
                match (
                    &(1 | 1 << 1 | 1 << 2),
                    &from_str_truncate::<TestFlags>("A\n|\tB\r\n|   C ").unwrap().bits(),
                ) {
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
                match (
                    &(1 | 1 << 1 | 1 << 2),
                    &from_str_truncate::<TestFlags>("A|B|C").unwrap().bits(),
                ) {
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
                match (&0, &from_str_truncate::<TestFlags>("0x8").unwrap().bits()) {
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
                match (&1, &from_str_truncate::<TestFlags>("A | 0x8").unwrap().bits()) {
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
                match (
                    &(1 | 1 << 1),
                    &from_str_truncate::<TestFlags>("0x1 | 0x8 | B").unwrap().bits(),
                ) {
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
                match (
                    &(1 | 1 << 1),
                    &from_str_truncate::<TestUnicode>("一 | 二").unwrap().bits(),
                ) {
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
        mod to_writer_truncate {
            use super::*;
            extern crate test;
            #[rustc_test_marker = "tests::parser::to_writer_truncate::cases"]
            #[doc(hidden)]
            pub const cases: test::TestDescAndFn = test::TestDescAndFn {
                desc: test::TestDesc {
                    name: test::StaticTestName(
                        "tests::parser::to_writer_truncate::cases",
                    ),
                    ignore: false,
                    ignore_message: ::core::option::Option::None,
                    source_file: "src/tests/parser.rs",
                    start_line: 206usize,
                    start_col: 8usize,
                    end_line: 206usize,
                    end_col: 13usize,
                    compile_fail: false,
                    no_run: false,
                    should_panic: test::ShouldPanic::No,
                    test_type: test::TestType::UnitTest,
                },
                testfn: test::StaticTestFn(
                    #[coverage(off)]
                    || test::assert_test_result(cases()),
                ),
            };
            fn cases() {
                match (&"", &write(TestFlags::empty())) {
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
                match (&"A", &write(TestFlags::A)) {
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
                match (&"A | B | C", &write(TestFlags::all())) {
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
                match (&"", &write(TestFlags::from_bits_retain(1 << 3))) {
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
                match (
                    &"A",
                    &write(TestFlags::A | TestFlags::from_bits_retain(1 << 3)),
                ) {
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
                match (&"", &write(TestZero::ZERO)) {
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
                match (&"ABC", &write(TestFlagsInvert::all())) {
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
                match (&"0x1", &write(TestOverlapping::from_bits_retain(1))) {
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
                match (&"A", &write(TestOverlappingFull::C)) {
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
                match (
                    &"A | D",
                    &write(TestOverlappingFull::C | TestOverlappingFull::D),
                ) {
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
            fn write<F: Flags>(value: F) -> String
            where
                F::Bits: crate::parser::WriteHex,
            {
                let mut s = String::new();
                to_writer_truncate(&value, &mut s).unwrap();
                s
            }
        }
        mod from_str_strict {
            use super::*;
            extern crate test;
            #[rustc_test_marker = "tests::parser::from_str_strict::valid"]
            #[doc(hidden)]
            pub const valid: test::TestDescAndFn = test::TestDescAndFn {
                desc: test::TestDesc {
                    name: test::StaticTestName("tests::parser::from_str_strict::valid"),
                    ignore: false,
                    ignore_message: ::core::option::Option::None,
                    source_file: "src/tests/parser.rs",
                    start_line: 244usize,
                    start_col: 8usize,
                    end_line: 244usize,
                    end_col: 13usize,
                    compile_fail: false,
                    no_run: false,
                    should_panic: test::ShouldPanic::No,
                    test_type: test::TestType::UnitTest,
                },
                testfn: test::StaticTestFn(
                    #[coverage(off)]
                    || test::assert_test_result(valid()),
                ),
            };
            fn valid() {
                match (&0, &from_str_strict::<TestFlags>("").unwrap().bits()) {
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
                match (&1, &from_str_strict::<TestFlags>("A").unwrap().bits()) {
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
                match (&1, &from_str_strict::<TestFlags>(" A ").unwrap().bits()) {
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
                match (
                    &(1 | 1 << 1 | 1 << 2),
                    &from_str_strict::<TestFlags>("A | B | C").unwrap().bits(),
                ) {
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
                match (
                    &(1 | 1 << 1 | 1 << 2),
                    &from_str_strict::<TestFlags>("A\n|\tB\r\n|   C ").unwrap().bits(),
                ) {
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
                match (
                    &(1 | 1 << 1 | 1 << 2),
                    &from_str_strict::<TestFlags>("A|B|C").unwrap().bits(),
                ) {
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
                match (
                    &(1 | 1 << 1),
                    &from_str_strict::<TestUnicode>("一 | 二").unwrap().bits(),
                ) {
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
            #[rustc_test_marker = "tests::parser::from_str_strict::invalid"]
            #[doc(hidden)]
            pub const invalid: test::TestDescAndFn = test::TestDescAndFn {
                desc: test::TestDesc {
                    name: test::StaticTestName(
                        "tests::parser::from_str_strict::invalid",
                    ),
                    ignore: false,
                    ignore_message: ::core::option::Option::None,
                    source_file: "src/tests/parser.rs",
                    start_line: 271usize,
                    start_col: 8usize,
                    end_line: 271usize,
                    end_col: 15usize,
                    compile_fail: false,
                    no_run: false,
                    should_panic: test::ShouldPanic::No,
                    test_type: test::TestType::UnitTest,
                },
                testfn: test::StaticTestFn(
                    #[coverage(off)]
                    || test::assert_test_result(invalid()),
                ),
            };
            fn invalid() {
                if !from_str_strict::<TestFlags>("a")
                    .unwrap_err()
                    .to_string()
                    .starts_with("unrecognized named flag")
                {
                    ::core::panicking::panic(
                        "assertion failed: from_str_strict::<TestFlags>(\"a\").unwrap_err().to_string().starts_with(\"unrecognized named flag\")",
                    )
                }
                if !from_str_strict::<TestFlags>("A & B")
                    .unwrap_err()
                    .to_string()
                    .starts_with("unrecognized named flag")
                {
                    ::core::panicking::panic(
                        "assertion failed: from_str_strict::<TestFlags>(\"A & B\").unwrap_err().to_string().starts_with(\"unrecognized named flag\")",
                    )
                }
                if !from_str_strict::<TestFlags>("0x1")
                    .unwrap_err()
                    .to_string()
                    .starts_with("invalid hex flag")
                {
                    ::core::panicking::panic(
                        "assertion failed: from_str_strict::<TestFlags>(\"0x1\").unwrap_err().to_string().starts_with(\"invalid hex flag\")",
                    )
                }
                if !from_str_strict::<TestFlags>("0xg")
                    .unwrap_err()
                    .to_string()
                    .starts_with("invalid hex flag")
                {
                    ::core::panicking::panic(
                        "assertion failed: from_str_strict::<TestFlags>(\"0xg\").unwrap_err().to_string().starts_with(\"invalid hex flag\")",
                    )
                }
                if !from_str_strict::<TestFlags>("0xffffffffffff")
                    .unwrap_err()
                    .to_string()
                    .starts_with("invalid hex flag")
                {
                    ::core::panicking::panic(
                        "assertion failed: from_str_strict::<TestFlags>(\"0xffffffffffff\").unwrap_err().to_string().starts_with(\"invalid hex flag\")",
                    )
                }
            }
        }
        mod to_writer_strict {
            use super::*;
            extern crate test;
            #[rustc_test_marker = "tests::parser::to_writer_strict::cases"]
            #[doc(hidden)]
            pub const cases: test::TestDescAndFn = test::TestDescAndFn {
                desc: test::TestDesc {
                    name: test::StaticTestName("tests::parser::to_writer_strict::cases"),
                    ignore: false,
                    ignore_message: ::core::option::Option::None,
                    source_file: "src/tests/parser.rs",
                    start_line: 300usize,
                    start_col: 8usize,
                    end_line: 300usize,
                    end_col: 13usize,
                    compile_fail: false,
                    no_run: false,
                    should_panic: test::ShouldPanic::No,
                    test_type: test::TestType::UnitTest,
                },
                testfn: test::StaticTestFn(
                    #[coverage(off)]
                    || test::assert_test_result(cases()),
                ),
            };
            fn cases() {
                match (&"", &write(TestFlags::empty())) {
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
                match (&"A", &write(TestFlags::A)) {
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
                match (&"A | B | C", &write(TestFlags::all())) {
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
                match (&"", &write(TestFlags::from_bits_retain(1 << 3))) {
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
                match (
                    &"A",
                    &write(TestFlags::A | TestFlags::from_bits_retain(1 << 3)),
                ) {
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
                match (&"", &write(TestZero::ZERO)) {
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
                match (&"ABC", &write(TestFlagsInvert::all())) {
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
                match (&"", &write(TestOverlapping::from_bits_retain(1))) {
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
                match (&"A", &write(TestOverlappingFull::C)) {
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
                match (
                    &"A | D",
                    &write(TestOverlappingFull::C | TestOverlappingFull::D),
                ) {
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
            fn write<F: Flags>(value: F) -> String
            where
                F::Bits: crate::parser::WriteHex,
            {
                let mut s = String::new();
                to_writer_strict(&value, &mut s).unwrap();
                s
            }
        }
    }
    mod remove {
        use super::*;
        use crate::Flags;
        extern crate test;
        #[rustc_test_marker = "tests::remove::cases"]
        #[doc(hidden)]
        pub const cases: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::remove::cases"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/remove.rs",
                start_line: 6usize,
                start_col: 4usize,
                end_line: 6usize,
                end_col: 9usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(cases()),
            ),
        };
        fn cases() {
            case(
                TestFlags::empty(),
                &[
                    (TestFlags::A, 0),
                    (TestFlags::empty(), 0),
                    (TestFlags::from_bits_retain(1 << 3), 0),
                ],
                TestFlags::remove,
                TestFlags::set,
            );
            case(
                TestFlags::A,
                &[(TestFlags::A, 0), (TestFlags::empty(), 1), (TestFlags::B, 1)],
                TestFlags::remove,
                TestFlags::set,
            );
            case(
                TestFlags::ABC,
                &[
                    (TestFlags::A, 1 << 1 | 1 << 2),
                    (TestFlags::A | TestFlags::C, 1 << 1),
                ],
                TestFlags::remove,
                TestFlags::set,
            );
        }
        #[track_caller]
        fn case<T: Flags + std::fmt::Debug + Copy>(
            value: T,
            inputs: &[(T, T::Bits)],
            mut inherent_remove: impl FnMut(&mut T, T),
            mut inherent_set: impl FnMut(&mut T, T, bool),
        )
        where
            T::Bits: std::fmt::Debug + PartialEq + Copy,
        {
            for (input, expected) in inputs {
                match (
                    &*expected,
                    &{
                        let mut value = value;
                        inherent_remove(&mut value, *input);
                        value
                    }
                        .bits(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("{0:?}.remove({1:?})", value, input),
                                ),
                            );
                        }
                    }
                };
                match (
                    &*expected,
                    &{
                        let mut value = value;
                        Flags::remove(&mut value, *input);
                        value
                    }
                        .bits(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("Flags::remove({0:?}, {1:?})", value, input),
                                ),
                            );
                        }
                    }
                };
                match (
                    &*expected,
                    &{
                        let mut value = value;
                        inherent_set(&mut value, *input, false);
                        value
                    }
                        .bits(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("{0:?}.set({1:?}, false)", value, input),
                                ),
                            );
                        }
                    }
                };
                match (
                    &*expected,
                    &{
                        let mut value = value;
                        Flags::set(&mut value, *input, false);
                        value
                    }
                        .bits(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!(
                                        "Flags::set({0:?}, {1:?}, false)",
                                        value,
                                        input,
                                    ),
                                ),
                            );
                        }
                    }
                };
            }
        }
    }
    mod symmetric_difference {
        use super::*;
        use crate::Flags;
        extern crate test;
        #[rustc_test_marker = "tests::symmetric_difference::cases"]
        #[doc(hidden)]
        pub const cases: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::symmetric_difference::cases"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/symmetric_difference.rs",
                start_line: 6usize,
                start_col: 4usize,
                end_line: 6usize,
                end_col: 9usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(cases()),
            ),
        };
        fn cases() {
            case(
                TestFlags::empty(),
                &[
                    (TestFlags::empty(), 0),
                    (TestFlags::all(), 1 | 1 << 1 | 1 << 2),
                    (TestFlags::from_bits_retain(1 << 3), 1 << 3),
                ],
                TestFlags::symmetric_difference,
                TestFlags::toggle,
            );
            case(
                TestFlags::A,
                &[
                    (TestFlags::empty(), 1),
                    (TestFlags::A, 0),
                    (TestFlags::all(), 1 << 1 | 1 << 2),
                ],
                TestFlags::symmetric_difference,
                TestFlags::toggle,
            );
            case(
                TestFlags::A | TestFlags::B | TestFlags::from_bits_retain(1 << 3),
                &[
                    (TestFlags::ABC, 1 << 2 | 1 << 3),
                    (TestFlags::from_bits_retain(1 << 3), 1 | 1 << 1),
                ],
                TestFlags::symmetric_difference,
                TestFlags::toggle,
            );
        }
        #[track_caller]
        fn case<
            T: Flags + std::fmt::Debug + std::ops::BitXor<Output = T>
                + std::ops::BitXorAssign + Copy,
        >(
            value: T,
            inputs: &[(T, T::Bits)],
            mut inherent_sym_diff: impl FnMut(T, T) -> T,
            mut inherent_toggle: impl FnMut(&mut T, T),
        )
        where
            T::Bits: std::fmt::Debug + PartialEq + Copy,
        {
            for (input, expected) in inputs {
                match (&*expected, &inherent_sym_diff(value, *input).bits()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!(
                                        "{0:?}.symmetric_difference({1:?})",
                                        value,
                                        input,
                                    ),
                                ),
                            );
                        }
                    }
                };
                match (&*expected, &Flags::symmetric_difference(value, *input).bits()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!(
                                        "Flags::symmetric_difference({0:?}, {1:?})",
                                        value,
                                        input,
                                    ),
                                ),
                            );
                        }
                    }
                };
                match (&*expected, &(value ^ *input).bits()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("{0:?} ^ {1:?}", value, input),
                                ),
                            );
                        }
                    }
                };
                match (
                    &*expected,
                    &{
                        let mut value = value;
                        value ^= *input;
                        value
                    }
                        .bits(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("{0:?} ^= {1:?}", value, input),
                                ),
                            );
                        }
                    }
                };
                match (
                    &*expected,
                    &{
                        let mut value = value;
                        inherent_toggle(&mut value, *input);
                        value
                    }
                        .bits(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("{0:?}.toggle({1:?})", value, input),
                                ),
                            );
                        }
                    }
                };
                match (
                    &*expected,
                    &{
                        let mut value = value;
                        Flags::toggle(&mut value, *input);
                        value
                    }
                        .bits(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("{0:?}.toggle({1:?})", value, input),
                                ),
                            );
                        }
                    }
                };
            }
        }
    }
    mod union {
        use super::*;
        use crate::Flags;
        extern crate test;
        #[rustc_test_marker = "tests::union::cases"]
        #[doc(hidden)]
        pub const cases: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("tests::union::cases"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests/union.rs",
                start_line: 6usize,
                start_col: 4usize,
                end_line: 6usize,
                end_col: 9usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(cases()),
            ),
        };
        fn cases() {
            case(
                TestFlags::empty(),
                &[
                    (TestFlags::A, 1),
                    (TestFlags::all(), 1 | 1 << 1 | 1 << 2),
                    (TestFlags::empty(), 0),
                    (TestFlags::from_bits_retain(1 << 3), 1 << 3),
                ],
                TestFlags::union,
            );
            case(
                TestFlags::A | TestFlags::C,
                &[
                    (TestFlags::A | TestFlags::B, 1 | 1 << 1 | 1 << 2),
                    (TestFlags::A, 1 | 1 << 2),
                ],
                TestFlags::union,
            );
        }
        #[track_caller]
        fn case<
            T: Flags + std::fmt::Debug + std::ops::BitOr<Output = T>
                + std::ops::BitOrAssign + Copy,
        >(value: T, inputs: &[(T, T::Bits)], mut inherent: impl FnMut(T, T) -> T)
        where
            T::Bits: std::fmt::Debug + PartialEq + Copy,
        {
            for (input, expected) in inputs {
                match (&*expected, &inherent(value, *input).bits()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("{0:?}.union({1:?})", value, input),
                                ),
                            );
                        }
                    }
                };
                match (&*expected, &Flags::union(value, *input).bits()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("Flags::union({0:?}, {1:?})", value, input),
                                ),
                            );
                        }
                    }
                };
                match (&*expected, &(value | *input).bits()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("{0:?} | {1:?}", value, input),
                                ),
                            );
                        }
                    }
                };
                match (
                    &*expected,
                    &{
                        let mut value = value;
                        value |= *input;
                        value
                    }
                        .bits(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("{0:?} |= {1:?}", value, input),
                                ),
                            );
                        }
                    }
                };
            }
        }
    }
    pub struct TestFlags(<TestFlags as crate::__private::PublicFlags>::Internal);
    #[automatically_derived]
    impl ::core::fmt::Debug for TestFlags {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_tuple_field1_finish(f, "TestFlags", &&self.0)
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for TestFlags {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for TestFlags {
        #[inline]
        fn eq(&self, other: &TestFlags) -> bool {
            self.0 == other.0
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for TestFlags {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<
                <TestFlags as crate::__private::PublicFlags>::Internal,
            >;
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for TestFlags {
        #[inline]
        fn partial_cmp(
            &self,
            other: &TestFlags,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for TestFlags {
        #[inline]
        fn cmp(&self, other: &TestFlags) -> ::core::cmp::Ordering {
            ::core::cmp::Ord::cmp(&self.0, &other.0)
        }
    }
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for TestFlags {}
    #[automatically_derived]
    impl ::core::clone::Clone for TestFlags {
        #[inline]
        fn clone(&self) -> TestFlags {
            let _: ::core::clone::AssertParamIsClone<
                <TestFlags as crate::__private::PublicFlags>::Internal,
            >;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for TestFlags {}
    impl TestFlags {
        /// 1
        #[allow(deprecated, non_upper_case_globals)]
        pub const A: Self = Self::from_bits_retain(1);
        /// 1 << 1
        #[allow(deprecated, non_upper_case_globals)]
        pub const B: Self = Self::from_bits_retain(1 << 1);
        /// 1 << 2
        #[allow(deprecated, non_upper_case_globals)]
        pub const C: Self = Self::from_bits_retain(1 << 2);
        /// 1 | (1 << 1) | (1 << 2)
        #[allow(deprecated, non_upper_case_globals)]
        pub const ABC: Self = Self::from_bits_retain(
            Self::A.bits() | Self::B.bits() | Self::C.bits(),
        );
    }
    impl crate::Flags for TestFlags {
        const FLAGS: &'static [crate::Flag<TestFlags>] = &[
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("A", TestFlags::A)
            },
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("B", TestFlags::B)
            },
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("C", TestFlags::C)
            },
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("ABC", TestFlags::ABC)
            },
        ];
        type Bits = u8;
        fn bits(&self) -> u8 {
            TestFlags::bits(self)
        }
        fn from_bits_retain(bits: u8) -> TestFlags {
            TestFlags::from_bits_retain(bits)
        }
    }
    #[allow(
        dead_code,
        deprecated,
        unused_doc_comments,
        unused_attributes,
        unused_mut,
        unused_imports,
        non_upper_case_globals,
        clippy::assign_op_pattern,
        clippy::indexing_slicing,
        clippy::same_name_method,
        clippy::iter_without_into_iter,
    )]
    const _: () = {
        #[repr(transparent)]
        pub struct InternalBitFlags(u8);
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::clone::Clone for InternalBitFlags {
            #[inline]
            fn clone(&self) -> InternalBitFlags {
                let _: ::core::clone::AssertParamIsClone<u8>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for InternalBitFlags {
            #[inline]
            fn eq(&self, other: &InternalBitFlags) -> bool {
                self.0 == other.0
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for InternalBitFlags {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<u8>;
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for InternalBitFlags {
            #[inline]
            fn partial_cmp(
                &self,
                other: &InternalBitFlags,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for InternalBitFlags {
            #[inline]
            fn cmp(&self, other: &InternalBitFlags) -> ::core::cmp::Ordering {
                ::core::cmp::Ord::cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for InternalBitFlags {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                ::core::hash::Hash::hash(&self.0, state)
            }
        }
        impl crate::__private::PublicFlags for TestFlags {
            type Primitive = u8;
            type Internal = InternalBitFlags;
        }
        impl crate::__private::core::default::Default for InternalBitFlags {
            #[inline]
            fn default() -> Self {
                InternalBitFlags::empty()
            }
        }
        impl crate::__private::core::fmt::Debug for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter<'_>,
            ) -> crate::__private::core::fmt::Result {
                if self.is_empty() {
                    f.write_fmt(format_args!("{0:#x}", <u8 as crate::Bits>::EMPTY))
                } else {
                    crate::__private::core::fmt::Display::fmt(self, f)
                }
            }
        }
        impl crate::__private::core::fmt::Display for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter<'_>,
            ) -> crate::__private::core::fmt::Result {
                crate::parser::to_writer(&TestFlags(*self), f)
            }
        }
        impl crate::__private::core::str::FromStr for InternalBitFlags {
            type Err = crate::parser::ParseError;
            fn from_str(
                s: &str,
            ) -> crate::__private::core::result::Result<Self, Self::Err> {
                crate::parser::from_str::<TestFlags>(s).map(|flags| flags.0)
            }
        }
        impl crate::__private::core::convert::AsRef<u8> for InternalBitFlags {
            fn as_ref(&self) -> &u8 {
                &self.0
            }
        }
        impl crate::__private::core::convert::From<u8> for InternalBitFlags {
            fn from(bits: u8) -> Self {
                Self::from_bits_retain(bits)
            }
        }
        #[allow(dead_code, deprecated, unused_attributes)]
        impl InternalBitFlags {
            /// Get a flags value with all bits unset.
            #[inline]
            pub const fn empty() -> Self {
                { Self(<u8 as crate::Bits>::EMPTY) }
            }
            /// Get a flags value with all known bits set.
            #[inline]
            pub const fn all() -> Self {
                {
                    let mut truncated = <u8 as crate::Bits>::EMPTY;
                    let mut i = 0;
                    {
                        {
                            let flag = <TestFlags as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    {
                        {
                            let flag = <TestFlags as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    {
                        {
                            let flag = <TestFlags as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    {
                        {
                            let flag = <TestFlags as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    let _ = i;
                    Self::from_bits_retain(truncated)
                }
            }
            /// Get the underlying bits value.
            ///
            /// The returned value is exactly the bits set in this flags value.
            #[inline]
            pub const fn bits(&self) -> u8 {
                let f = self;
                { f.0 }
            }
            /// Convert from a bits value.
            ///
            /// This method will return `None` if any unknown bits are set.
            #[inline]
            pub const fn from_bits(
                bits: u8,
            ) -> crate::__private::core::option::Option<Self> {
                let bits = bits;
                {
                    let truncated = Self::from_bits_truncate(bits).0;
                    if truncated == bits {
                        crate::__private::core::option::Option::Some(Self(bits))
                    } else {
                        crate::__private::core::option::Option::None
                    }
                }
            }
            /// Convert from a bits value, unsetting any unknown bits.
            #[inline]
            pub const fn from_bits_truncate(bits: u8) -> Self {
                let bits = bits;
                { Self(bits & Self::all().bits()) }
            }
            /// Convert from a bits value exactly.
            #[inline]
            pub const fn from_bits_retain(bits: u8) -> Self {
                let bits = bits;
                { Self(bits) }
            }
            /// Get a flags value with the bits of a flag with the given name set.
            ///
            /// This method will return `None` if `name` is empty or doesn't
            /// correspond to any named flag.
            #[inline]
            pub fn from_name(
                name: &str,
            ) -> crate::__private::core::option::Option<Self> {
                let name = name;
                {
                    {
                        if name == "A" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestFlags::A.bits()),
                            );
                        }
                    };
                    {
                        if name == "B" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestFlags::B.bits()),
                            );
                        }
                    };
                    {
                        if name == "C" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestFlags::C.bits()),
                            );
                        }
                    };
                    {
                        if name == "ABC" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestFlags::ABC.bits()),
                            );
                        }
                    };
                    let _ = name;
                    crate::__private::core::option::Option::None
                }
            }
            /// Whether all bits in this flags value are unset.
            #[inline]
            pub const fn is_empty(&self) -> bool {
                let f = self;
                { f.bits() == <u8 as crate::Bits>::EMPTY }
            }
            /// Whether all known bits in this flags value are set.
            #[inline]
            pub const fn is_all(&self) -> bool {
                let f = self;
                { Self::all().bits() | f.bits() == f.bits() }
            }
            /// Whether any set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn intersects(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.bits() & other.bits() != <u8 as crate::Bits>::EMPTY }
            }
            /// Whether all set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn contains(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.bits() & other.bits() == other.bits() }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            pub fn insert(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).union(other);
                }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `remove` won't truncate `other`, but the `!` operator will.
            #[inline]
            pub fn remove(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).difference(other);
                }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            pub fn toggle(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).symmetric_difference(other);
                }
            }
            /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
            #[inline]
            pub fn set(&mut self, other: Self, value: bool) {
                let f = self;
                let other = other;
                let value = value;
                {
                    if value {
                        f.insert(other);
                    } else {
                        f.remove(other);
                    }
                }
            }
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() & other.bits()) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn union(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() | other.bits()) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            #[must_use]
            pub const fn difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() & !other.bits()) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn symmetric_difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() ^ other.bits()) }
            }
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            #[must_use]
            pub const fn complement(self) -> Self {
                let f = self;
                { Self::from_bits_truncate(!f.bits()) }
            }
        }
        impl crate::__private::core::fmt::Binary for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Binary::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::Octal for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Octal::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::LowerHex for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::LowerHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::UpperHex for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::UpperHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::ops::BitOr for InternalBitFlags {
            type Output = Self;
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor(self, other: InternalBitFlags) -> Self {
                self.union(other)
            }
        }
        impl crate::__private::core::ops::BitOrAssign for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor_assign(&mut self, other: Self) {
                self.insert(other);
            }
        }
        impl crate::__private::core::ops::BitXor for InternalBitFlags {
            type Output = Self;
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor(self, other: Self) -> Self {
                self.symmetric_difference(other)
            }
        }
        impl crate::__private::core::ops::BitXorAssign for InternalBitFlags {
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor_assign(&mut self, other: Self) {
                self.toggle(other);
            }
        }
        impl crate::__private::core::ops::BitAnd for InternalBitFlags {
            type Output = Self;
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand(self, other: Self) -> Self {
                self.intersection(other)
            }
        }
        impl crate::__private::core::ops::BitAndAssign for InternalBitFlags {
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand_assign(&mut self, other: Self) {
                *self = Self::from_bits_retain(self.bits()).intersection(other);
            }
        }
        impl crate::__private::core::ops::Sub for InternalBitFlags {
            type Output = Self;
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub(self, other: Self) -> Self {
                self.difference(other)
            }
        }
        impl crate::__private::core::ops::SubAssign for InternalBitFlags {
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                self.remove(other);
            }
        }
        impl crate::__private::core::ops::Not for InternalBitFlags {
            type Output = Self;
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            fn not(self) -> Self {
                self.complement()
            }
        }
        impl crate::__private::core::iter::Extend<InternalBitFlags>
        for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn extend<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                &mut self,
                iterator: T,
            ) {
                for item in iterator {
                    self.insert(item)
                }
            }
        }
        impl crate::__private::core::iter::FromIterator<InternalBitFlags>
        for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn from_iter<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                iterator: T,
            ) -> Self {
                use crate::__private::core::iter::Extend;
                let mut result = Self::empty();
                result.extend(iterator);
                result
            }
        }
        impl InternalBitFlags {
            /// Yield a set of contained flags values.
            ///
            /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
            /// will be yielded together as a final flags value.
            #[inline]
            pub const fn iter(&self) -> crate::iter::Iter<TestFlags> {
                crate::iter::Iter::__private_const_new(
                    <TestFlags as crate::Flags>::FLAGS,
                    TestFlags::from_bits_retain(self.bits()),
                    TestFlags::from_bits_retain(self.bits()),
                )
            }
            /// Yield a set of contained named flags values.
            ///
            /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
            /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
            #[inline]
            pub const fn iter_names(&self) -> crate::iter::IterNames<TestFlags> {
                crate::iter::IterNames::__private_const_new(
                    <TestFlags as crate::Flags>::FLAGS,
                    TestFlags::from_bits_retain(self.bits()),
                    TestFlags::from_bits_retain(self.bits()),
                )
            }
        }
        impl crate::__private::core::iter::IntoIterator for InternalBitFlags {
            type Item = TestFlags;
            type IntoIter = crate::iter::Iter<TestFlags>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
        impl InternalBitFlags {
            /// Returns a mutable reference to the raw value of the flags currently stored.
            #[inline]
            pub fn bits_mut(&mut self) -> &mut u8 {
                &mut self.0
            }
        }
        #[allow(dead_code, deprecated, unused_attributes)]
        impl TestFlags {
            /// Get a flags value with all bits unset.
            #[inline]
            pub const fn empty() -> Self {
                { Self(InternalBitFlags::empty()) }
            }
            /// Get a flags value with all known bits set.
            #[inline]
            pub const fn all() -> Self {
                { Self(InternalBitFlags::all()) }
            }
            /// Get the underlying bits value.
            ///
            /// The returned value is exactly the bits set in this flags value.
            #[inline]
            pub const fn bits(&self) -> u8 {
                let f = self;
                { f.0.bits() }
            }
            /// Convert from a bits value.
            ///
            /// This method will return `None` if any unknown bits are set.
            #[inline]
            pub const fn from_bits(
                bits: u8,
            ) -> crate::__private::core::option::Option<Self> {
                let bits = bits;
                {
                    match InternalBitFlags::from_bits(bits) {
                        crate::__private::core::option::Option::Some(bits) => {
                            crate::__private::core::option::Option::Some(Self(bits))
                        }
                        crate::__private::core::option::Option::None => {
                            crate::__private::core::option::Option::None
                        }
                    }
                }
            }
            /// Convert from a bits value, unsetting any unknown bits.
            #[inline]
            pub const fn from_bits_truncate(bits: u8) -> Self {
                let bits = bits;
                { Self(InternalBitFlags::from_bits_truncate(bits)) }
            }
            /// Convert from a bits value exactly.
            #[inline]
            pub const fn from_bits_retain(bits: u8) -> Self {
                let bits = bits;
                { Self(InternalBitFlags::from_bits_retain(bits)) }
            }
            /// Get a flags value with the bits of a flag with the given name set.
            ///
            /// This method will return `None` if `name` is empty or doesn't
            /// correspond to any named flag.
            #[inline]
            pub fn from_name(
                name: &str,
            ) -> crate::__private::core::option::Option<Self> {
                let name = name;
                {
                    match InternalBitFlags::from_name(name) {
                        crate::__private::core::option::Option::Some(bits) => {
                            crate::__private::core::option::Option::Some(Self(bits))
                        }
                        crate::__private::core::option::Option::None => {
                            crate::__private::core::option::Option::None
                        }
                    }
                }
            }
            /// Whether all bits in this flags value are unset.
            #[inline]
            pub const fn is_empty(&self) -> bool {
                let f = self;
                { f.0.is_empty() }
            }
            /// Whether all known bits in this flags value are set.
            #[inline]
            pub const fn is_all(&self) -> bool {
                let f = self;
                { f.0.is_all() }
            }
            /// Whether any set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn intersects(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.0.intersects(other.0) }
            }
            /// Whether all set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn contains(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.0.contains(other.0) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            pub fn insert(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.insert(other.0) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `remove` won't truncate `other`, but the `!` operator will.
            #[inline]
            pub fn remove(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.remove(other.0) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            pub fn toggle(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.toggle(other.0) }
            }
            /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
            #[inline]
            pub fn set(&mut self, other: Self, value: bool) {
                let f = self;
                let other = other;
                let value = value;
                { f.0.set(other.0, value) }
            }
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.intersection(other.0)) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn union(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.union(other.0)) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            #[must_use]
            pub const fn difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.difference(other.0)) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn symmetric_difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.symmetric_difference(other.0)) }
            }
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            #[must_use]
            pub const fn complement(self) -> Self {
                let f = self;
                { Self(f.0.complement()) }
            }
        }
        impl crate::__private::core::fmt::Binary for TestFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Binary::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::Octal for TestFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Octal::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::LowerHex for TestFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::LowerHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::UpperHex for TestFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::UpperHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::ops::BitOr for TestFlags {
            type Output = Self;
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor(self, other: TestFlags) -> Self {
                self.union(other)
            }
        }
        impl crate::__private::core::ops::BitOrAssign for TestFlags {
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor_assign(&mut self, other: Self) {
                self.insert(other);
            }
        }
        impl crate::__private::core::ops::BitXor for TestFlags {
            type Output = Self;
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor(self, other: Self) -> Self {
                self.symmetric_difference(other)
            }
        }
        impl crate::__private::core::ops::BitXorAssign for TestFlags {
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor_assign(&mut self, other: Self) {
                self.toggle(other);
            }
        }
        impl crate::__private::core::ops::BitAnd for TestFlags {
            type Output = Self;
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand(self, other: Self) -> Self {
                self.intersection(other)
            }
        }
        impl crate::__private::core::ops::BitAndAssign for TestFlags {
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand_assign(&mut self, other: Self) {
                *self = Self::from_bits_retain(self.bits()).intersection(other);
            }
        }
        impl crate::__private::core::ops::Sub for TestFlags {
            type Output = Self;
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub(self, other: Self) -> Self {
                self.difference(other)
            }
        }
        impl crate::__private::core::ops::SubAssign for TestFlags {
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                self.remove(other);
            }
        }
        impl crate::__private::core::ops::Not for TestFlags {
            type Output = Self;
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            fn not(self) -> Self {
                self.complement()
            }
        }
        impl crate::__private::core::iter::Extend<TestFlags> for TestFlags {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn extend<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                &mut self,
                iterator: T,
            ) {
                for item in iterator {
                    self.insert(item)
                }
            }
        }
        impl crate::__private::core::iter::FromIterator<TestFlags> for TestFlags {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn from_iter<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                iterator: T,
            ) -> Self {
                use crate::__private::core::iter::Extend;
                let mut result = Self::empty();
                result.extend(iterator);
                result
            }
        }
        impl TestFlags {
            /// Yield a set of contained flags values.
            ///
            /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
            /// will be yielded together as a final flags value.
            #[inline]
            pub const fn iter(&self) -> crate::iter::Iter<TestFlags> {
                crate::iter::Iter::__private_const_new(
                    <TestFlags as crate::Flags>::FLAGS,
                    TestFlags::from_bits_retain(self.bits()),
                    TestFlags::from_bits_retain(self.bits()),
                )
            }
            /// Yield a set of contained named flags values.
            ///
            /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
            /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
            #[inline]
            pub const fn iter_names(&self) -> crate::iter::IterNames<TestFlags> {
                crate::iter::IterNames::__private_const_new(
                    <TestFlags as crate::Flags>::FLAGS,
                    TestFlags::from_bits_retain(self.bits()),
                    TestFlags::from_bits_retain(self.bits()),
                )
            }
        }
        impl crate::__private::core::iter::IntoIterator for TestFlags {
            type Item = TestFlags;
            type IntoIter = crate::iter::Iter<TestFlags>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
    };
    pub struct TestFlagsInvert(
        <TestFlagsInvert as crate::__private::PublicFlags>::Internal,
    );
    #[automatically_derived]
    impl ::core::fmt::Debug for TestFlagsInvert {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_tuple_field1_finish(
                f,
                "TestFlagsInvert",
                &&self.0,
            )
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for TestFlagsInvert {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for TestFlagsInvert {
        #[inline]
        fn eq(&self, other: &TestFlagsInvert) -> bool {
            self.0 == other.0
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for TestFlagsInvert {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<
                <TestFlagsInvert as crate::__private::PublicFlags>::Internal,
            >;
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for TestFlagsInvert {
        #[inline]
        fn partial_cmp(
            &self,
            other: &TestFlagsInvert,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for TestFlagsInvert {
        #[inline]
        fn cmp(&self, other: &TestFlagsInvert) -> ::core::cmp::Ordering {
            ::core::cmp::Ord::cmp(&self.0, &other.0)
        }
    }
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for TestFlagsInvert {}
    #[automatically_derived]
    impl ::core::clone::Clone for TestFlagsInvert {
        #[inline]
        fn clone(&self) -> TestFlagsInvert {
            let _: ::core::clone::AssertParamIsClone<
                <TestFlagsInvert as crate::__private::PublicFlags>::Internal,
            >;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for TestFlagsInvert {}
    impl TestFlagsInvert {
        /// 1 | (1 << 1) | (1 << 2)
        #[allow(deprecated, non_upper_case_globals)]
        pub const ABC: Self = Self::from_bits_retain(
            Self::A.bits() | Self::B.bits() | Self::C.bits(),
        );
        /// 1
        #[allow(deprecated, non_upper_case_globals)]
        pub const A: Self = Self::from_bits_retain(1);
        /// 1 << 1
        #[allow(deprecated, non_upper_case_globals)]
        pub const B: Self = Self::from_bits_retain(1 << 1);
        /// 1 << 2
        #[allow(deprecated, non_upper_case_globals)]
        pub const C: Self = Self::from_bits_retain(1 << 2);
    }
    impl crate::Flags for TestFlagsInvert {
        const FLAGS: &'static [crate::Flag<TestFlagsInvert>] = &[
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("ABC", TestFlagsInvert::ABC)
            },
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("A", TestFlagsInvert::A)
            },
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("B", TestFlagsInvert::B)
            },
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("C", TestFlagsInvert::C)
            },
        ];
        type Bits = u8;
        fn bits(&self) -> u8 {
            TestFlagsInvert::bits(self)
        }
        fn from_bits_retain(bits: u8) -> TestFlagsInvert {
            TestFlagsInvert::from_bits_retain(bits)
        }
    }
    #[allow(
        dead_code,
        deprecated,
        unused_doc_comments,
        unused_attributes,
        unused_mut,
        unused_imports,
        non_upper_case_globals,
        clippy::assign_op_pattern,
        clippy::indexing_slicing,
        clippy::same_name_method,
        clippy::iter_without_into_iter,
    )]
    const _: () = {
        #[repr(transparent)]
        pub struct InternalBitFlags(u8);
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::clone::Clone for InternalBitFlags {
            #[inline]
            fn clone(&self) -> InternalBitFlags {
                let _: ::core::clone::AssertParamIsClone<u8>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for InternalBitFlags {
            #[inline]
            fn eq(&self, other: &InternalBitFlags) -> bool {
                self.0 == other.0
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for InternalBitFlags {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<u8>;
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for InternalBitFlags {
            #[inline]
            fn partial_cmp(
                &self,
                other: &InternalBitFlags,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for InternalBitFlags {
            #[inline]
            fn cmp(&self, other: &InternalBitFlags) -> ::core::cmp::Ordering {
                ::core::cmp::Ord::cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for InternalBitFlags {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                ::core::hash::Hash::hash(&self.0, state)
            }
        }
        impl crate::__private::PublicFlags for TestFlagsInvert {
            type Primitive = u8;
            type Internal = InternalBitFlags;
        }
        impl crate::__private::core::default::Default for InternalBitFlags {
            #[inline]
            fn default() -> Self {
                InternalBitFlags::empty()
            }
        }
        impl crate::__private::core::fmt::Debug for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter<'_>,
            ) -> crate::__private::core::fmt::Result {
                if self.is_empty() {
                    f.write_fmt(format_args!("{0:#x}", <u8 as crate::Bits>::EMPTY))
                } else {
                    crate::__private::core::fmt::Display::fmt(self, f)
                }
            }
        }
        impl crate::__private::core::fmt::Display for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter<'_>,
            ) -> crate::__private::core::fmt::Result {
                crate::parser::to_writer(&TestFlagsInvert(*self), f)
            }
        }
        impl crate::__private::core::str::FromStr for InternalBitFlags {
            type Err = crate::parser::ParseError;
            fn from_str(
                s: &str,
            ) -> crate::__private::core::result::Result<Self, Self::Err> {
                crate::parser::from_str::<TestFlagsInvert>(s).map(|flags| flags.0)
            }
        }
        impl crate::__private::core::convert::AsRef<u8> for InternalBitFlags {
            fn as_ref(&self) -> &u8 {
                &self.0
            }
        }
        impl crate::__private::core::convert::From<u8> for InternalBitFlags {
            fn from(bits: u8) -> Self {
                Self::from_bits_retain(bits)
            }
        }
        #[allow(dead_code, deprecated, unused_attributes)]
        impl InternalBitFlags {
            /// Get a flags value with all bits unset.
            #[inline]
            pub const fn empty() -> Self {
                { Self(<u8 as crate::Bits>::EMPTY) }
            }
            /// Get a flags value with all known bits set.
            #[inline]
            pub const fn all() -> Self {
                {
                    let mut truncated = <u8 as crate::Bits>::EMPTY;
                    let mut i = 0;
                    {
                        {
                            let flag = <TestFlagsInvert as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    {
                        {
                            let flag = <TestFlagsInvert as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    {
                        {
                            let flag = <TestFlagsInvert as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    {
                        {
                            let flag = <TestFlagsInvert as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    let _ = i;
                    Self::from_bits_retain(truncated)
                }
            }
            /// Get the underlying bits value.
            ///
            /// The returned value is exactly the bits set in this flags value.
            #[inline]
            pub const fn bits(&self) -> u8 {
                let f = self;
                { f.0 }
            }
            /// Convert from a bits value.
            ///
            /// This method will return `None` if any unknown bits are set.
            #[inline]
            pub const fn from_bits(
                bits: u8,
            ) -> crate::__private::core::option::Option<Self> {
                let bits = bits;
                {
                    let truncated = Self::from_bits_truncate(bits).0;
                    if truncated == bits {
                        crate::__private::core::option::Option::Some(Self(bits))
                    } else {
                        crate::__private::core::option::Option::None
                    }
                }
            }
            /// Convert from a bits value, unsetting any unknown bits.
            #[inline]
            pub const fn from_bits_truncate(bits: u8) -> Self {
                let bits = bits;
                { Self(bits & Self::all().bits()) }
            }
            /// Convert from a bits value exactly.
            #[inline]
            pub const fn from_bits_retain(bits: u8) -> Self {
                let bits = bits;
                { Self(bits) }
            }
            /// Get a flags value with the bits of a flag with the given name set.
            ///
            /// This method will return `None` if `name` is empty or doesn't
            /// correspond to any named flag.
            #[inline]
            pub fn from_name(
                name: &str,
            ) -> crate::__private::core::option::Option<Self> {
                let name = name;
                {
                    {
                        if name == "ABC" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestFlagsInvert::ABC.bits()),
                            );
                        }
                    };
                    {
                        if name == "A" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestFlagsInvert::A.bits()),
                            );
                        }
                    };
                    {
                        if name == "B" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestFlagsInvert::B.bits()),
                            );
                        }
                    };
                    {
                        if name == "C" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestFlagsInvert::C.bits()),
                            );
                        }
                    };
                    let _ = name;
                    crate::__private::core::option::Option::None
                }
            }
            /// Whether all bits in this flags value are unset.
            #[inline]
            pub const fn is_empty(&self) -> bool {
                let f = self;
                { f.bits() == <u8 as crate::Bits>::EMPTY }
            }
            /// Whether all known bits in this flags value are set.
            #[inline]
            pub const fn is_all(&self) -> bool {
                let f = self;
                { Self::all().bits() | f.bits() == f.bits() }
            }
            /// Whether any set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn intersects(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.bits() & other.bits() != <u8 as crate::Bits>::EMPTY }
            }
            /// Whether all set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn contains(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.bits() & other.bits() == other.bits() }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            pub fn insert(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).union(other);
                }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `remove` won't truncate `other`, but the `!` operator will.
            #[inline]
            pub fn remove(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).difference(other);
                }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            pub fn toggle(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).symmetric_difference(other);
                }
            }
            /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
            #[inline]
            pub fn set(&mut self, other: Self, value: bool) {
                let f = self;
                let other = other;
                let value = value;
                {
                    if value {
                        f.insert(other);
                    } else {
                        f.remove(other);
                    }
                }
            }
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() & other.bits()) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn union(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() | other.bits()) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            #[must_use]
            pub const fn difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() & !other.bits()) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn symmetric_difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() ^ other.bits()) }
            }
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            #[must_use]
            pub const fn complement(self) -> Self {
                let f = self;
                { Self::from_bits_truncate(!f.bits()) }
            }
        }
        impl crate::__private::core::fmt::Binary for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Binary::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::Octal for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Octal::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::LowerHex for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::LowerHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::UpperHex for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::UpperHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::ops::BitOr for InternalBitFlags {
            type Output = Self;
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor(self, other: InternalBitFlags) -> Self {
                self.union(other)
            }
        }
        impl crate::__private::core::ops::BitOrAssign for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor_assign(&mut self, other: Self) {
                self.insert(other);
            }
        }
        impl crate::__private::core::ops::BitXor for InternalBitFlags {
            type Output = Self;
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor(self, other: Self) -> Self {
                self.symmetric_difference(other)
            }
        }
        impl crate::__private::core::ops::BitXorAssign for InternalBitFlags {
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor_assign(&mut self, other: Self) {
                self.toggle(other);
            }
        }
        impl crate::__private::core::ops::BitAnd for InternalBitFlags {
            type Output = Self;
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand(self, other: Self) -> Self {
                self.intersection(other)
            }
        }
        impl crate::__private::core::ops::BitAndAssign for InternalBitFlags {
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand_assign(&mut self, other: Self) {
                *self = Self::from_bits_retain(self.bits()).intersection(other);
            }
        }
        impl crate::__private::core::ops::Sub for InternalBitFlags {
            type Output = Self;
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub(self, other: Self) -> Self {
                self.difference(other)
            }
        }
        impl crate::__private::core::ops::SubAssign for InternalBitFlags {
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                self.remove(other);
            }
        }
        impl crate::__private::core::ops::Not for InternalBitFlags {
            type Output = Self;
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            fn not(self) -> Self {
                self.complement()
            }
        }
        impl crate::__private::core::iter::Extend<InternalBitFlags>
        for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn extend<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                &mut self,
                iterator: T,
            ) {
                for item in iterator {
                    self.insert(item)
                }
            }
        }
        impl crate::__private::core::iter::FromIterator<InternalBitFlags>
        for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn from_iter<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                iterator: T,
            ) -> Self {
                use crate::__private::core::iter::Extend;
                let mut result = Self::empty();
                result.extend(iterator);
                result
            }
        }
        impl InternalBitFlags {
            /// Yield a set of contained flags values.
            ///
            /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
            /// will be yielded together as a final flags value.
            #[inline]
            pub const fn iter(&self) -> crate::iter::Iter<TestFlagsInvert> {
                crate::iter::Iter::__private_const_new(
                    <TestFlagsInvert as crate::Flags>::FLAGS,
                    TestFlagsInvert::from_bits_retain(self.bits()),
                    TestFlagsInvert::from_bits_retain(self.bits()),
                )
            }
            /// Yield a set of contained named flags values.
            ///
            /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
            /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
            #[inline]
            pub const fn iter_names(&self) -> crate::iter::IterNames<TestFlagsInvert> {
                crate::iter::IterNames::__private_const_new(
                    <TestFlagsInvert as crate::Flags>::FLAGS,
                    TestFlagsInvert::from_bits_retain(self.bits()),
                    TestFlagsInvert::from_bits_retain(self.bits()),
                )
            }
        }
        impl crate::__private::core::iter::IntoIterator for InternalBitFlags {
            type Item = TestFlagsInvert;
            type IntoIter = crate::iter::Iter<TestFlagsInvert>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
        impl InternalBitFlags {
            /// Returns a mutable reference to the raw value of the flags currently stored.
            #[inline]
            pub fn bits_mut(&mut self) -> &mut u8 {
                &mut self.0
            }
        }
        #[allow(dead_code, deprecated, unused_attributes)]
        impl TestFlagsInvert {
            /// Get a flags value with all bits unset.
            #[inline]
            pub const fn empty() -> Self {
                { Self(InternalBitFlags::empty()) }
            }
            /// Get a flags value with all known bits set.
            #[inline]
            pub const fn all() -> Self {
                { Self(InternalBitFlags::all()) }
            }
            /// Get the underlying bits value.
            ///
            /// The returned value is exactly the bits set in this flags value.
            #[inline]
            pub const fn bits(&self) -> u8 {
                let f = self;
                { f.0.bits() }
            }
            /// Convert from a bits value.
            ///
            /// This method will return `None` if any unknown bits are set.
            #[inline]
            pub const fn from_bits(
                bits: u8,
            ) -> crate::__private::core::option::Option<Self> {
                let bits = bits;
                {
                    match InternalBitFlags::from_bits(bits) {
                        crate::__private::core::option::Option::Some(bits) => {
                            crate::__private::core::option::Option::Some(Self(bits))
                        }
                        crate::__private::core::option::Option::None => {
                            crate::__private::core::option::Option::None
                        }
                    }
                }
            }
            /// Convert from a bits value, unsetting any unknown bits.
            #[inline]
            pub const fn from_bits_truncate(bits: u8) -> Self {
                let bits = bits;
                { Self(InternalBitFlags::from_bits_truncate(bits)) }
            }
            /// Convert from a bits value exactly.
            #[inline]
            pub const fn from_bits_retain(bits: u8) -> Self {
                let bits = bits;
                { Self(InternalBitFlags::from_bits_retain(bits)) }
            }
            /// Get a flags value with the bits of a flag with the given name set.
            ///
            /// This method will return `None` if `name` is empty or doesn't
            /// correspond to any named flag.
            #[inline]
            pub fn from_name(
                name: &str,
            ) -> crate::__private::core::option::Option<Self> {
                let name = name;
                {
                    match InternalBitFlags::from_name(name) {
                        crate::__private::core::option::Option::Some(bits) => {
                            crate::__private::core::option::Option::Some(Self(bits))
                        }
                        crate::__private::core::option::Option::None => {
                            crate::__private::core::option::Option::None
                        }
                    }
                }
            }
            /// Whether all bits in this flags value are unset.
            #[inline]
            pub const fn is_empty(&self) -> bool {
                let f = self;
                { f.0.is_empty() }
            }
            /// Whether all known bits in this flags value are set.
            #[inline]
            pub const fn is_all(&self) -> bool {
                let f = self;
                { f.0.is_all() }
            }
            /// Whether any set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn intersects(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.0.intersects(other.0) }
            }
            /// Whether all set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn contains(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.0.contains(other.0) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            pub fn insert(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.insert(other.0) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `remove` won't truncate `other`, but the `!` operator will.
            #[inline]
            pub fn remove(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.remove(other.0) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            pub fn toggle(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.toggle(other.0) }
            }
            /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
            #[inline]
            pub fn set(&mut self, other: Self, value: bool) {
                let f = self;
                let other = other;
                let value = value;
                { f.0.set(other.0, value) }
            }
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.intersection(other.0)) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn union(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.union(other.0)) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            #[must_use]
            pub const fn difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.difference(other.0)) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn symmetric_difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.symmetric_difference(other.0)) }
            }
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            #[must_use]
            pub const fn complement(self) -> Self {
                let f = self;
                { Self(f.0.complement()) }
            }
        }
        impl crate::__private::core::fmt::Binary for TestFlagsInvert {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Binary::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::Octal for TestFlagsInvert {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Octal::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::LowerHex for TestFlagsInvert {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::LowerHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::UpperHex for TestFlagsInvert {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::UpperHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::ops::BitOr for TestFlagsInvert {
            type Output = Self;
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor(self, other: TestFlagsInvert) -> Self {
                self.union(other)
            }
        }
        impl crate::__private::core::ops::BitOrAssign for TestFlagsInvert {
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor_assign(&mut self, other: Self) {
                self.insert(other);
            }
        }
        impl crate::__private::core::ops::BitXor for TestFlagsInvert {
            type Output = Self;
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor(self, other: Self) -> Self {
                self.symmetric_difference(other)
            }
        }
        impl crate::__private::core::ops::BitXorAssign for TestFlagsInvert {
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor_assign(&mut self, other: Self) {
                self.toggle(other);
            }
        }
        impl crate::__private::core::ops::BitAnd for TestFlagsInvert {
            type Output = Self;
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand(self, other: Self) -> Self {
                self.intersection(other)
            }
        }
        impl crate::__private::core::ops::BitAndAssign for TestFlagsInvert {
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand_assign(&mut self, other: Self) {
                *self = Self::from_bits_retain(self.bits()).intersection(other);
            }
        }
        impl crate::__private::core::ops::Sub for TestFlagsInvert {
            type Output = Self;
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub(self, other: Self) -> Self {
                self.difference(other)
            }
        }
        impl crate::__private::core::ops::SubAssign for TestFlagsInvert {
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                self.remove(other);
            }
        }
        impl crate::__private::core::ops::Not for TestFlagsInvert {
            type Output = Self;
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            fn not(self) -> Self {
                self.complement()
            }
        }
        impl crate::__private::core::iter::Extend<TestFlagsInvert> for TestFlagsInvert {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn extend<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                &mut self,
                iterator: T,
            ) {
                for item in iterator {
                    self.insert(item)
                }
            }
        }
        impl crate::__private::core::iter::FromIterator<TestFlagsInvert>
        for TestFlagsInvert {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn from_iter<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                iterator: T,
            ) -> Self {
                use crate::__private::core::iter::Extend;
                let mut result = Self::empty();
                result.extend(iterator);
                result
            }
        }
        impl TestFlagsInvert {
            /// Yield a set of contained flags values.
            ///
            /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
            /// will be yielded together as a final flags value.
            #[inline]
            pub const fn iter(&self) -> crate::iter::Iter<TestFlagsInvert> {
                crate::iter::Iter::__private_const_new(
                    <TestFlagsInvert as crate::Flags>::FLAGS,
                    TestFlagsInvert::from_bits_retain(self.bits()),
                    TestFlagsInvert::from_bits_retain(self.bits()),
                )
            }
            /// Yield a set of contained named flags values.
            ///
            /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
            /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
            #[inline]
            pub const fn iter_names(&self) -> crate::iter::IterNames<TestFlagsInvert> {
                crate::iter::IterNames::__private_const_new(
                    <TestFlagsInvert as crate::Flags>::FLAGS,
                    TestFlagsInvert::from_bits_retain(self.bits()),
                    TestFlagsInvert::from_bits_retain(self.bits()),
                )
            }
        }
        impl crate::__private::core::iter::IntoIterator for TestFlagsInvert {
            type Item = TestFlagsInvert;
            type IntoIter = crate::iter::Iter<TestFlagsInvert>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
    };
    pub struct TestZero(<TestZero as crate::__private::PublicFlags>::Internal);
    #[automatically_derived]
    impl ::core::fmt::Debug for TestZero {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_tuple_field1_finish(f, "TestZero", &&self.0)
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for TestZero {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for TestZero {
        #[inline]
        fn eq(&self, other: &TestZero) -> bool {
            self.0 == other.0
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for TestZero {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<
                <TestZero as crate::__private::PublicFlags>::Internal,
            >;
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for TestZero {
        #[inline]
        fn partial_cmp(
            &self,
            other: &TestZero,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for TestZero {
        #[inline]
        fn cmp(&self, other: &TestZero) -> ::core::cmp::Ordering {
            ::core::cmp::Ord::cmp(&self.0, &other.0)
        }
    }
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for TestZero {}
    #[automatically_derived]
    impl ::core::clone::Clone for TestZero {
        #[inline]
        fn clone(&self) -> TestZero {
            let _: ::core::clone::AssertParamIsClone<
                <TestZero as crate::__private::PublicFlags>::Internal,
            >;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for TestZero {}
    impl TestZero {
        /// 0
        #[allow(deprecated, non_upper_case_globals)]
        pub const ZERO: Self = Self::from_bits_retain(0);
    }
    impl crate::Flags for TestZero {
        const FLAGS: &'static [crate::Flag<TestZero>] = &[
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("ZERO", TestZero::ZERO)
            },
        ];
        type Bits = u8;
        fn bits(&self) -> u8 {
            TestZero::bits(self)
        }
        fn from_bits_retain(bits: u8) -> TestZero {
            TestZero::from_bits_retain(bits)
        }
    }
    #[allow(
        dead_code,
        deprecated,
        unused_doc_comments,
        unused_attributes,
        unused_mut,
        unused_imports,
        non_upper_case_globals,
        clippy::assign_op_pattern,
        clippy::indexing_slicing,
        clippy::same_name_method,
        clippy::iter_without_into_iter,
    )]
    const _: () = {
        #[repr(transparent)]
        pub struct InternalBitFlags(u8);
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::clone::Clone for InternalBitFlags {
            #[inline]
            fn clone(&self) -> InternalBitFlags {
                let _: ::core::clone::AssertParamIsClone<u8>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for InternalBitFlags {
            #[inline]
            fn eq(&self, other: &InternalBitFlags) -> bool {
                self.0 == other.0
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for InternalBitFlags {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<u8>;
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for InternalBitFlags {
            #[inline]
            fn partial_cmp(
                &self,
                other: &InternalBitFlags,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for InternalBitFlags {
            #[inline]
            fn cmp(&self, other: &InternalBitFlags) -> ::core::cmp::Ordering {
                ::core::cmp::Ord::cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for InternalBitFlags {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                ::core::hash::Hash::hash(&self.0, state)
            }
        }
        impl crate::__private::PublicFlags for TestZero {
            type Primitive = u8;
            type Internal = InternalBitFlags;
        }
        impl crate::__private::core::default::Default for InternalBitFlags {
            #[inline]
            fn default() -> Self {
                InternalBitFlags::empty()
            }
        }
        impl crate::__private::core::fmt::Debug for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter<'_>,
            ) -> crate::__private::core::fmt::Result {
                if self.is_empty() {
                    f.write_fmt(format_args!("{0:#x}", <u8 as crate::Bits>::EMPTY))
                } else {
                    crate::__private::core::fmt::Display::fmt(self, f)
                }
            }
        }
        impl crate::__private::core::fmt::Display for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter<'_>,
            ) -> crate::__private::core::fmt::Result {
                crate::parser::to_writer(&TestZero(*self), f)
            }
        }
        impl crate::__private::core::str::FromStr for InternalBitFlags {
            type Err = crate::parser::ParseError;
            fn from_str(
                s: &str,
            ) -> crate::__private::core::result::Result<Self, Self::Err> {
                crate::parser::from_str::<TestZero>(s).map(|flags| flags.0)
            }
        }
        impl crate::__private::core::convert::AsRef<u8> for InternalBitFlags {
            fn as_ref(&self) -> &u8 {
                &self.0
            }
        }
        impl crate::__private::core::convert::From<u8> for InternalBitFlags {
            fn from(bits: u8) -> Self {
                Self::from_bits_retain(bits)
            }
        }
        #[allow(dead_code, deprecated, unused_attributes)]
        impl InternalBitFlags {
            /// Get a flags value with all bits unset.
            #[inline]
            pub const fn empty() -> Self {
                { Self(<u8 as crate::Bits>::EMPTY) }
            }
            /// Get a flags value with all known bits set.
            #[inline]
            pub const fn all() -> Self {
                {
                    let mut truncated = <u8 as crate::Bits>::EMPTY;
                    let mut i = 0;
                    {
                        {
                            let flag = <TestZero as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    let _ = i;
                    Self::from_bits_retain(truncated)
                }
            }
            /// Get the underlying bits value.
            ///
            /// The returned value is exactly the bits set in this flags value.
            #[inline]
            pub const fn bits(&self) -> u8 {
                let f = self;
                { f.0 }
            }
            /// Convert from a bits value.
            ///
            /// This method will return `None` if any unknown bits are set.
            #[inline]
            pub const fn from_bits(
                bits: u8,
            ) -> crate::__private::core::option::Option<Self> {
                let bits = bits;
                {
                    let truncated = Self::from_bits_truncate(bits).0;
                    if truncated == bits {
                        crate::__private::core::option::Option::Some(Self(bits))
                    } else {
                        crate::__private::core::option::Option::None
                    }
                }
            }
            /// Convert from a bits value, unsetting any unknown bits.
            #[inline]
            pub const fn from_bits_truncate(bits: u8) -> Self {
                let bits = bits;
                { Self(bits & Self::all().bits()) }
            }
            /// Convert from a bits value exactly.
            #[inline]
            pub const fn from_bits_retain(bits: u8) -> Self {
                let bits = bits;
                { Self(bits) }
            }
            /// Get a flags value with the bits of a flag with the given name set.
            ///
            /// This method will return `None` if `name` is empty or doesn't
            /// correspond to any named flag.
            #[inline]
            pub fn from_name(
                name: &str,
            ) -> crate::__private::core::option::Option<Self> {
                let name = name;
                {
                    {
                        if name == "ZERO" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestZero::ZERO.bits()),
                            );
                        }
                    };
                    let _ = name;
                    crate::__private::core::option::Option::None
                }
            }
            /// Whether all bits in this flags value are unset.
            #[inline]
            pub const fn is_empty(&self) -> bool {
                let f = self;
                { f.bits() == <u8 as crate::Bits>::EMPTY }
            }
            /// Whether all known bits in this flags value are set.
            #[inline]
            pub const fn is_all(&self) -> bool {
                let f = self;
                { Self::all().bits() | f.bits() == f.bits() }
            }
            /// Whether any set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn intersects(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.bits() & other.bits() != <u8 as crate::Bits>::EMPTY }
            }
            /// Whether all set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn contains(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.bits() & other.bits() == other.bits() }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            pub fn insert(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).union(other);
                }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `remove` won't truncate `other`, but the `!` operator will.
            #[inline]
            pub fn remove(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).difference(other);
                }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            pub fn toggle(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).symmetric_difference(other);
                }
            }
            /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
            #[inline]
            pub fn set(&mut self, other: Self, value: bool) {
                let f = self;
                let other = other;
                let value = value;
                {
                    if value {
                        f.insert(other);
                    } else {
                        f.remove(other);
                    }
                }
            }
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() & other.bits()) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn union(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() | other.bits()) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            #[must_use]
            pub const fn difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() & !other.bits()) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn symmetric_difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() ^ other.bits()) }
            }
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            #[must_use]
            pub const fn complement(self) -> Self {
                let f = self;
                { Self::from_bits_truncate(!f.bits()) }
            }
        }
        impl crate::__private::core::fmt::Binary for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Binary::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::Octal for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Octal::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::LowerHex for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::LowerHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::UpperHex for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::UpperHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::ops::BitOr for InternalBitFlags {
            type Output = Self;
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor(self, other: InternalBitFlags) -> Self {
                self.union(other)
            }
        }
        impl crate::__private::core::ops::BitOrAssign for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor_assign(&mut self, other: Self) {
                self.insert(other);
            }
        }
        impl crate::__private::core::ops::BitXor for InternalBitFlags {
            type Output = Self;
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor(self, other: Self) -> Self {
                self.symmetric_difference(other)
            }
        }
        impl crate::__private::core::ops::BitXorAssign for InternalBitFlags {
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor_assign(&mut self, other: Self) {
                self.toggle(other);
            }
        }
        impl crate::__private::core::ops::BitAnd for InternalBitFlags {
            type Output = Self;
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand(self, other: Self) -> Self {
                self.intersection(other)
            }
        }
        impl crate::__private::core::ops::BitAndAssign for InternalBitFlags {
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand_assign(&mut self, other: Self) {
                *self = Self::from_bits_retain(self.bits()).intersection(other);
            }
        }
        impl crate::__private::core::ops::Sub for InternalBitFlags {
            type Output = Self;
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub(self, other: Self) -> Self {
                self.difference(other)
            }
        }
        impl crate::__private::core::ops::SubAssign for InternalBitFlags {
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                self.remove(other);
            }
        }
        impl crate::__private::core::ops::Not for InternalBitFlags {
            type Output = Self;
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            fn not(self) -> Self {
                self.complement()
            }
        }
        impl crate::__private::core::iter::Extend<InternalBitFlags>
        for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn extend<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                &mut self,
                iterator: T,
            ) {
                for item in iterator {
                    self.insert(item)
                }
            }
        }
        impl crate::__private::core::iter::FromIterator<InternalBitFlags>
        for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn from_iter<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                iterator: T,
            ) -> Self {
                use crate::__private::core::iter::Extend;
                let mut result = Self::empty();
                result.extend(iterator);
                result
            }
        }
        impl InternalBitFlags {
            /// Yield a set of contained flags values.
            ///
            /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
            /// will be yielded together as a final flags value.
            #[inline]
            pub const fn iter(&self) -> crate::iter::Iter<TestZero> {
                crate::iter::Iter::__private_const_new(
                    <TestZero as crate::Flags>::FLAGS,
                    TestZero::from_bits_retain(self.bits()),
                    TestZero::from_bits_retain(self.bits()),
                )
            }
            /// Yield a set of contained named flags values.
            ///
            /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
            /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
            #[inline]
            pub const fn iter_names(&self) -> crate::iter::IterNames<TestZero> {
                crate::iter::IterNames::__private_const_new(
                    <TestZero as crate::Flags>::FLAGS,
                    TestZero::from_bits_retain(self.bits()),
                    TestZero::from_bits_retain(self.bits()),
                )
            }
        }
        impl crate::__private::core::iter::IntoIterator for InternalBitFlags {
            type Item = TestZero;
            type IntoIter = crate::iter::Iter<TestZero>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
        impl InternalBitFlags {
            /// Returns a mutable reference to the raw value of the flags currently stored.
            #[inline]
            pub fn bits_mut(&mut self) -> &mut u8 {
                &mut self.0
            }
        }
        #[allow(dead_code, deprecated, unused_attributes)]
        impl TestZero {
            /// Get a flags value with all bits unset.
            #[inline]
            pub const fn empty() -> Self {
                { Self(InternalBitFlags::empty()) }
            }
            /// Get a flags value with all known bits set.
            #[inline]
            pub const fn all() -> Self {
                { Self(InternalBitFlags::all()) }
            }
            /// Get the underlying bits value.
            ///
            /// The returned value is exactly the bits set in this flags value.
            #[inline]
            pub const fn bits(&self) -> u8 {
                let f = self;
                { f.0.bits() }
            }
            /// Convert from a bits value.
            ///
            /// This method will return `None` if any unknown bits are set.
            #[inline]
            pub const fn from_bits(
                bits: u8,
            ) -> crate::__private::core::option::Option<Self> {
                let bits = bits;
                {
                    match InternalBitFlags::from_bits(bits) {
                        crate::__private::core::option::Option::Some(bits) => {
                            crate::__private::core::option::Option::Some(Self(bits))
                        }
                        crate::__private::core::option::Option::None => {
                            crate::__private::core::option::Option::None
                        }
                    }
                }
            }
            /// Convert from a bits value, unsetting any unknown bits.
            #[inline]
            pub const fn from_bits_truncate(bits: u8) -> Self {
                let bits = bits;
                { Self(InternalBitFlags::from_bits_truncate(bits)) }
            }
            /// Convert from a bits value exactly.
            #[inline]
            pub const fn from_bits_retain(bits: u8) -> Self {
                let bits = bits;
                { Self(InternalBitFlags::from_bits_retain(bits)) }
            }
            /// Get a flags value with the bits of a flag with the given name set.
            ///
            /// This method will return `None` if `name` is empty or doesn't
            /// correspond to any named flag.
            #[inline]
            pub fn from_name(
                name: &str,
            ) -> crate::__private::core::option::Option<Self> {
                let name = name;
                {
                    match InternalBitFlags::from_name(name) {
                        crate::__private::core::option::Option::Some(bits) => {
                            crate::__private::core::option::Option::Some(Self(bits))
                        }
                        crate::__private::core::option::Option::None => {
                            crate::__private::core::option::Option::None
                        }
                    }
                }
            }
            /// Whether all bits in this flags value are unset.
            #[inline]
            pub const fn is_empty(&self) -> bool {
                let f = self;
                { f.0.is_empty() }
            }
            /// Whether all known bits in this flags value are set.
            #[inline]
            pub const fn is_all(&self) -> bool {
                let f = self;
                { f.0.is_all() }
            }
            /// Whether any set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn intersects(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.0.intersects(other.0) }
            }
            /// Whether all set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn contains(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.0.contains(other.0) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            pub fn insert(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.insert(other.0) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `remove` won't truncate `other`, but the `!` operator will.
            #[inline]
            pub fn remove(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.remove(other.0) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            pub fn toggle(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.toggle(other.0) }
            }
            /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
            #[inline]
            pub fn set(&mut self, other: Self, value: bool) {
                let f = self;
                let other = other;
                let value = value;
                { f.0.set(other.0, value) }
            }
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.intersection(other.0)) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn union(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.union(other.0)) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            #[must_use]
            pub const fn difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.difference(other.0)) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn symmetric_difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.symmetric_difference(other.0)) }
            }
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            #[must_use]
            pub const fn complement(self) -> Self {
                let f = self;
                { Self(f.0.complement()) }
            }
        }
        impl crate::__private::core::fmt::Binary for TestZero {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Binary::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::Octal for TestZero {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Octal::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::LowerHex for TestZero {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::LowerHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::UpperHex for TestZero {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::UpperHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::ops::BitOr for TestZero {
            type Output = Self;
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor(self, other: TestZero) -> Self {
                self.union(other)
            }
        }
        impl crate::__private::core::ops::BitOrAssign for TestZero {
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor_assign(&mut self, other: Self) {
                self.insert(other);
            }
        }
        impl crate::__private::core::ops::BitXor for TestZero {
            type Output = Self;
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor(self, other: Self) -> Self {
                self.symmetric_difference(other)
            }
        }
        impl crate::__private::core::ops::BitXorAssign for TestZero {
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor_assign(&mut self, other: Self) {
                self.toggle(other);
            }
        }
        impl crate::__private::core::ops::BitAnd for TestZero {
            type Output = Self;
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand(self, other: Self) -> Self {
                self.intersection(other)
            }
        }
        impl crate::__private::core::ops::BitAndAssign for TestZero {
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand_assign(&mut self, other: Self) {
                *self = Self::from_bits_retain(self.bits()).intersection(other);
            }
        }
        impl crate::__private::core::ops::Sub for TestZero {
            type Output = Self;
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub(self, other: Self) -> Self {
                self.difference(other)
            }
        }
        impl crate::__private::core::ops::SubAssign for TestZero {
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                self.remove(other);
            }
        }
        impl crate::__private::core::ops::Not for TestZero {
            type Output = Self;
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            fn not(self) -> Self {
                self.complement()
            }
        }
        impl crate::__private::core::iter::Extend<TestZero> for TestZero {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn extend<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                &mut self,
                iterator: T,
            ) {
                for item in iterator {
                    self.insert(item)
                }
            }
        }
        impl crate::__private::core::iter::FromIterator<TestZero> for TestZero {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn from_iter<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                iterator: T,
            ) -> Self {
                use crate::__private::core::iter::Extend;
                let mut result = Self::empty();
                result.extend(iterator);
                result
            }
        }
        impl TestZero {
            /// Yield a set of contained flags values.
            ///
            /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
            /// will be yielded together as a final flags value.
            #[inline]
            pub const fn iter(&self) -> crate::iter::Iter<TestZero> {
                crate::iter::Iter::__private_const_new(
                    <TestZero as crate::Flags>::FLAGS,
                    TestZero::from_bits_retain(self.bits()),
                    TestZero::from_bits_retain(self.bits()),
                )
            }
            /// Yield a set of contained named flags values.
            ///
            /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
            /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
            #[inline]
            pub const fn iter_names(&self) -> crate::iter::IterNames<TestZero> {
                crate::iter::IterNames::__private_const_new(
                    <TestZero as crate::Flags>::FLAGS,
                    TestZero::from_bits_retain(self.bits()),
                    TestZero::from_bits_retain(self.bits()),
                )
            }
        }
        impl crate::__private::core::iter::IntoIterator for TestZero {
            type Item = TestZero;
            type IntoIter = crate::iter::Iter<TestZero>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
    };
    pub struct TestZeroOne(<TestZeroOne as crate::__private::PublicFlags>::Internal);
    #[automatically_derived]
    impl ::core::fmt::Debug for TestZeroOne {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_tuple_field1_finish(f, "TestZeroOne", &&self.0)
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for TestZeroOne {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for TestZeroOne {
        #[inline]
        fn eq(&self, other: &TestZeroOne) -> bool {
            self.0 == other.0
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for TestZeroOne {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<
                <TestZeroOne as crate::__private::PublicFlags>::Internal,
            >;
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for TestZeroOne {
        #[inline]
        fn partial_cmp(
            &self,
            other: &TestZeroOne,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for TestZeroOne {
        #[inline]
        fn cmp(&self, other: &TestZeroOne) -> ::core::cmp::Ordering {
            ::core::cmp::Ord::cmp(&self.0, &other.0)
        }
    }
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for TestZeroOne {}
    #[automatically_derived]
    impl ::core::clone::Clone for TestZeroOne {
        #[inline]
        fn clone(&self) -> TestZeroOne {
            let _: ::core::clone::AssertParamIsClone<
                <TestZeroOne as crate::__private::PublicFlags>::Internal,
            >;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for TestZeroOne {}
    impl TestZeroOne {
        /// 0
        #[allow(deprecated, non_upper_case_globals)]
        pub const ZERO: Self = Self::from_bits_retain(0);
        /// 1
        #[allow(deprecated, non_upper_case_globals)]
        pub const ONE: Self = Self::from_bits_retain(1);
    }
    impl crate::Flags for TestZeroOne {
        const FLAGS: &'static [crate::Flag<TestZeroOne>] = &[
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("ZERO", TestZeroOne::ZERO)
            },
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("ONE", TestZeroOne::ONE)
            },
        ];
        type Bits = u8;
        fn bits(&self) -> u8 {
            TestZeroOne::bits(self)
        }
        fn from_bits_retain(bits: u8) -> TestZeroOne {
            TestZeroOne::from_bits_retain(bits)
        }
    }
    #[allow(
        dead_code,
        deprecated,
        unused_doc_comments,
        unused_attributes,
        unused_mut,
        unused_imports,
        non_upper_case_globals,
        clippy::assign_op_pattern,
        clippy::indexing_slicing,
        clippy::same_name_method,
        clippy::iter_without_into_iter,
    )]
    const _: () = {
        #[repr(transparent)]
        pub struct InternalBitFlags(u8);
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::clone::Clone for InternalBitFlags {
            #[inline]
            fn clone(&self) -> InternalBitFlags {
                let _: ::core::clone::AssertParamIsClone<u8>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for InternalBitFlags {
            #[inline]
            fn eq(&self, other: &InternalBitFlags) -> bool {
                self.0 == other.0
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for InternalBitFlags {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<u8>;
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for InternalBitFlags {
            #[inline]
            fn partial_cmp(
                &self,
                other: &InternalBitFlags,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for InternalBitFlags {
            #[inline]
            fn cmp(&self, other: &InternalBitFlags) -> ::core::cmp::Ordering {
                ::core::cmp::Ord::cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for InternalBitFlags {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                ::core::hash::Hash::hash(&self.0, state)
            }
        }
        impl crate::__private::PublicFlags for TestZeroOne {
            type Primitive = u8;
            type Internal = InternalBitFlags;
        }
        impl crate::__private::core::default::Default for InternalBitFlags {
            #[inline]
            fn default() -> Self {
                InternalBitFlags::empty()
            }
        }
        impl crate::__private::core::fmt::Debug for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter<'_>,
            ) -> crate::__private::core::fmt::Result {
                if self.is_empty() {
                    f.write_fmt(format_args!("{0:#x}", <u8 as crate::Bits>::EMPTY))
                } else {
                    crate::__private::core::fmt::Display::fmt(self, f)
                }
            }
        }
        impl crate::__private::core::fmt::Display for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter<'_>,
            ) -> crate::__private::core::fmt::Result {
                crate::parser::to_writer(&TestZeroOne(*self), f)
            }
        }
        impl crate::__private::core::str::FromStr for InternalBitFlags {
            type Err = crate::parser::ParseError;
            fn from_str(
                s: &str,
            ) -> crate::__private::core::result::Result<Self, Self::Err> {
                crate::parser::from_str::<TestZeroOne>(s).map(|flags| flags.0)
            }
        }
        impl crate::__private::core::convert::AsRef<u8> for InternalBitFlags {
            fn as_ref(&self) -> &u8 {
                &self.0
            }
        }
        impl crate::__private::core::convert::From<u8> for InternalBitFlags {
            fn from(bits: u8) -> Self {
                Self::from_bits_retain(bits)
            }
        }
        #[allow(dead_code, deprecated, unused_attributes)]
        impl InternalBitFlags {
            /// Get a flags value with all bits unset.
            #[inline]
            pub const fn empty() -> Self {
                { Self(<u8 as crate::Bits>::EMPTY) }
            }
            /// Get a flags value with all known bits set.
            #[inline]
            pub const fn all() -> Self {
                {
                    let mut truncated = <u8 as crate::Bits>::EMPTY;
                    let mut i = 0;
                    {
                        {
                            let flag = <TestZeroOne as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    {
                        {
                            let flag = <TestZeroOne as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    let _ = i;
                    Self::from_bits_retain(truncated)
                }
            }
            /// Get the underlying bits value.
            ///
            /// The returned value is exactly the bits set in this flags value.
            #[inline]
            pub const fn bits(&self) -> u8 {
                let f = self;
                { f.0 }
            }
            /// Convert from a bits value.
            ///
            /// This method will return `None` if any unknown bits are set.
            #[inline]
            pub const fn from_bits(
                bits: u8,
            ) -> crate::__private::core::option::Option<Self> {
                let bits = bits;
                {
                    let truncated = Self::from_bits_truncate(bits).0;
                    if truncated == bits {
                        crate::__private::core::option::Option::Some(Self(bits))
                    } else {
                        crate::__private::core::option::Option::None
                    }
                }
            }
            /// Convert from a bits value, unsetting any unknown bits.
            #[inline]
            pub const fn from_bits_truncate(bits: u8) -> Self {
                let bits = bits;
                { Self(bits & Self::all().bits()) }
            }
            /// Convert from a bits value exactly.
            #[inline]
            pub const fn from_bits_retain(bits: u8) -> Self {
                let bits = bits;
                { Self(bits) }
            }
            /// Get a flags value with the bits of a flag with the given name set.
            ///
            /// This method will return `None` if `name` is empty or doesn't
            /// correspond to any named flag.
            #[inline]
            pub fn from_name(
                name: &str,
            ) -> crate::__private::core::option::Option<Self> {
                let name = name;
                {
                    {
                        if name == "ZERO" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestZeroOne::ZERO.bits()),
                            );
                        }
                    };
                    {
                        if name == "ONE" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestZeroOne::ONE.bits()),
                            );
                        }
                    };
                    let _ = name;
                    crate::__private::core::option::Option::None
                }
            }
            /// Whether all bits in this flags value are unset.
            #[inline]
            pub const fn is_empty(&self) -> bool {
                let f = self;
                { f.bits() == <u8 as crate::Bits>::EMPTY }
            }
            /// Whether all known bits in this flags value are set.
            #[inline]
            pub const fn is_all(&self) -> bool {
                let f = self;
                { Self::all().bits() | f.bits() == f.bits() }
            }
            /// Whether any set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn intersects(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.bits() & other.bits() != <u8 as crate::Bits>::EMPTY }
            }
            /// Whether all set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn contains(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.bits() & other.bits() == other.bits() }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            pub fn insert(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).union(other);
                }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `remove` won't truncate `other`, but the `!` operator will.
            #[inline]
            pub fn remove(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).difference(other);
                }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            pub fn toggle(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).symmetric_difference(other);
                }
            }
            /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
            #[inline]
            pub fn set(&mut self, other: Self, value: bool) {
                let f = self;
                let other = other;
                let value = value;
                {
                    if value {
                        f.insert(other);
                    } else {
                        f.remove(other);
                    }
                }
            }
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() & other.bits()) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn union(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() | other.bits()) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            #[must_use]
            pub const fn difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() & !other.bits()) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn symmetric_difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() ^ other.bits()) }
            }
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            #[must_use]
            pub const fn complement(self) -> Self {
                let f = self;
                { Self::from_bits_truncate(!f.bits()) }
            }
        }
        impl crate::__private::core::fmt::Binary for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Binary::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::Octal for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Octal::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::LowerHex for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::LowerHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::UpperHex for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::UpperHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::ops::BitOr for InternalBitFlags {
            type Output = Self;
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor(self, other: InternalBitFlags) -> Self {
                self.union(other)
            }
        }
        impl crate::__private::core::ops::BitOrAssign for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor_assign(&mut self, other: Self) {
                self.insert(other);
            }
        }
        impl crate::__private::core::ops::BitXor for InternalBitFlags {
            type Output = Self;
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor(self, other: Self) -> Self {
                self.symmetric_difference(other)
            }
        }
        impl crate::__private::core::ops::BitXorAssign for InternalBitFlags {
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor_assign(&mut self, other: Self) {
                self.toggle(other);
            }
        }
        impl crate::__private::core::ops::BitAnd for InternalBitFlags {
            type Output = Self;
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand(self, other: Self) -> Self {
                self.intersection(other)
            }
        }
        impl crate::__private::core::ops::BitAndAssign for InternalBitFlags {
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand_assign(&mut self, other: Self) {
                *self = Self::from_bits_retain(self.bits()).intersection(other);
            }
        }
        impl crate::__private::core::ops::Sub for InternalBitFlags {
            type Output = Self;
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub(self, other: Self) -> Self {
                self.difference(other)
            }
        }
        impl crate::__private::core::ops::SubAssign for InternalBitFlags {
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                self.remove(other);
            }
        }
        impl crate::__private::core::ops::Not for InternalBitFlags {
            type Output = Self;
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            fn not(self) -> Self {
                self.complement()
            }
        }
        impl crate::__private::core::iter::Extend<InternalBitFlags>
        for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn extend<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                &mut self,
                iterator: T,
            ) {
                for item in iterator {
                    self.insert(item)
                }
            }
        }
        impl crate::__private::core::iter::FromIterator<InternalBitFlags>
        for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn from_iter<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                iterator: T,
            ) -> Self {
                use crate::__private::core::iter::Extend;
                let mut result = Self::empty();
                result.extend(iterator);
                result
            }
        }
        impl InternalBitFlags {
            /// Yield a set of contained flags values.
            ///
            /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
            /// will be yielded together as a final flags value.
            #[inline]
            pub const fn iter(&self) -> crate::iter::Iter<TestZeroOne> {
                crate::iter::Iter::__private_const_new(
                    <TestZeroOne as crate::Flags>::FLAGS,
                    TestZeroOne::from_bits_retain(self.bits()),
                    TestZeroOne::from_bits_retain(self.bits()),
                )
            }
            /// Yield a set of contained named flags values.
            ///
            /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
            /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
            #[inline]
            pub const fn iter_names(&self) -> crate::iter::IterNames<TestZeroOne> {
                crate::iter::IterNames::__private_const_new(
                    <TestZeroOne as crate::Flags>::FLAGS,
                    TestZeroOne::from_bits_retain(self.bits()),
                    TestZeroOne::from_bits_retain(self.bits()),
                )
            }
        }
        impl crate::__private::core::iter::IntoIterator for InternalBitFlags {
            type Item = TestZeroOne;
            type IntoIter = crate::iter::Iter<TestZeroOne>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
        impl InternalBitFlags {
            /// Returns a mutable reference to the raw value of the flags currently stored.
            #[inline]
            pub fn bits_mut(&mut self) -> &mut u8 {
                &mut self.0
            }
        }
        #[allow(dead_code, deprecated, unused_attributes)]
        impl TestZeroOne {
            /// Get a flags value with all bits unset.
            #[inline]
            pub const fn empty() -> Self {
                { Self(InternalBitFlags::empty()) }
            }
            /// Get a flags value with all known bits set.
            #[inline]
            pub const fn all() -> Self {
                { Self(InternalBitFlags::all()) }
            }
            /// Get the underlying bits value.
            ///
            /// The returned value is exactly the bits set in this flags value.
            #[inline]
            pub const fn bits(&self) -> u8 {
                let f = self;
                { f.0.bits() }
            }
            /// Convert from a bits value.
            ///
            /// This method will return `None` if any unknown bits are set.
            #[inline]
            pub const fn from_bits(
                bits: u8,
            ) -> crate::__private::core::option::Option<Self> {
                let bits = bits;
                {
                    match InternalBitFlags::from_bits(bits) {
                        crate::__private::core::option::Option::Some(bits) => {
                            crate::__private::core::option::Option::Some(Self(bits))
                        }
                        crate::__private::core::option::Option::None => {
                            crate::__private::core::option::Option::None
                        }
                    }
                }
            }
            /// Convert from a bits value, unsetting any unknown bits.
            #[inline]
            pub const fn from_bits_truncate(bits: u8) -> Self {
                let bits = bits;
                { Self(InternalBitFlags::from_bits_truncate(bits)) }
            }
            /// Convert from a bits value exactly.
            #[inline]
            pub const fn from_bits_retain(bits: u8) -> Self {
                let bits = bits;
                { Self(InternalBitFlags::from_bits_retain(bits)) }
            }
            /// Get a flags value with the bits of a flag with the given name set.
            ///
            /// This method will return `None` if `name` is empty or doesn't
            /// correspond to any named flag.
            #[inline]
            pub fn from_name(
                name: &str,
            ) -> crate::__private::core::option::Option<Self> {
                let name = name;
                {
                    match InternalBitFlags::from_name(name) {
                        crate::__private::core::option::Option::Some(bits) => {
                            crate::__private::core::option::Option::Some(Self(bits))
                        }
                        crate::__private::core::option::Option::None => {
                            crate::__private::core::option::Option::None
                        }
                    }
                }
            }
            /// Whether all bits in this flags value are unset.
            #[inline]
            pub const fn is_empty(&self) -> bool {
                let f = self;
                { f.0.is_empty() }
            }
            /// Whether all known bits in this flags value are set.
            #[inline]
            pub const fn is_all(&self) -> bool {
                let f = self;
                { f.0.is_all() }
            }
            /// Whether any set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn intersects(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.0.intersects(other.0) }
            }
            /// Whether all set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn contains(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.0.contains(other.0) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            pub fn insert(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.insert(other.0) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `remove` won't truncate `other`, but the `!` operator will.
            #[inline]
            pub fn remove(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.remove(other.0) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            pub fn toggle(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.toggle(other.0) }
            }
            /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
            #[inline]
            pub fn set(&mut self, other: Self, value: bool) {
                let f = self;
                let other = other;
                let value = value;
                { f.0.set(other.0, value) }
            }
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.intersection(other.0)) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn union(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.union(other.0)) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            #[must_use]
            pub const fn difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.difference(other.0)) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn symmetric_difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.symmetric_difference(other.0)) }
            }
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            #[must_use]
            pub const fn complement(self) -> Self {
                let f = self;
                { Self(f.0.complement()) }
            }
        }
        impl crate::__private::core::fmt::Binary for TestZeroOne {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Binary::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::Octal for TestZeroOne {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Octal::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::LowerHex for TestZeroOne {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::LowerHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::UpperHex for TestZeroOne {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::UpperHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::ops::BitOr for TestZeroOne {
            type Output = Self;
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor(self, other: TestZeroOne) -> Self {
                self.union(other)
            }
        }
        impl crate::__private::core::ops::BitOrAssign for TestZeroOne {
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor_assign(&mut self, other: Self) {
                self.insert(other);
            }
        }
        impl crate::__private::core::ops::BitXor for TestZeroOne {
            type Output = Self;
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor(self, other: Self) -> Self {
                self.symmetric_difference(other)
            }
        }
        impl crate::__private::core::ops::BitXorAssign for TestZeroOne {
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor_assign(&mut self, other: Self) {
                self.toggle(other);
            }
        }
        impl crate::__private::core::ops::BitAnd for TestZeroOne {
            type Output = Self;
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand(self, other: Self) -> Self {
                self.intersection(other)
            }
        }
        impl crate::__private::core::ops::BitAndAssign for TestZeroOne {
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand_assign(&mut self, other: Self) {
                *self = Self::from_bits_retain(self.bits()).intersection(other);
            }
        }
        impl crate::__private::core::ops::Sub for TestZeroOne {
            type Output = Self;
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub(self, other: Self) -> Self {
                self.difference(other)
            }
        }
        impl crate::__private::core::ops::SubAssign for TestZeroOne {
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                self.remove(other);
            }
        }
        impl crate::__private::core::ops::Not for TestZeroOne {
            type Output = Self;
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            fn not(self) -> Self {
                self.complement()
            }
        }
        impl crate::__private::core::iter::Extend<TestZeroOne> for TestZeroOne {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn extend<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                &mut self,
                iterator: T,
            ) {
                for item in iterator {
                    self.insert(item)
                }
            }
        }
        impl crate::__private::core::iter::FromIterator<TestZeroOne> for TestZeroOne {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn from_iter<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                iterator: T,
            ) -> Self {
                use crate::__private::core::iter::Extend;
                let mut result = Self::empty();
                result.extend(iterator);
                result
            }
        }
        impl TestZeroOne {
            /// Yield a set of contained flags values.
            ///
            /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
            /// will be yielded together as a final flags value.
            #[inline]
            pub const fn iter(&self) -> crate::iter::Iter<TestZeroOne> {
                crate::iter::Iter::__private_const_new(
                    <TestZeroOne as crate::Flags>::FLAGS,
                    TestZeroOne::from_bits_retain(self.bits()),
                    TestZeroOne::from_bits_retain(self.bits()),
                )
            }
            /// Yield a set of contained named flags values.
            ///
            /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
            /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
            #[inline]
            pub const fn iter_names(&self) -> crate::iter::IterNames<TestZeroOne> {
                crate::iter::IterNames::__private_const_new(
                    <TestZeroOne as crate::Flags>::FLAGS,
                    TestZeroOne::from_bits_retain(self.bits()),
                    TestZeroOne::from_bits_retain(self.bits()),
                )
            }
        }
        impl crate::__private::core::iter::IntoIterator for TestZeroOne {
            type Item = TestZeroOne;
            type IntoIter = crate::iter::Iter<TestZeroOne>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
    };
    pub struct TestUnicode(<TestUnicode as crate::__private::PublicFlags>::Internal);
    #[automatically_derived]
    impl ::core::fmt::Debug for TestUnicode {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_tuple_field1_finish(f, "TestUnicode", &&self.0)
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for TestUnicode {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for TestUnicode {
        #[inline]
        fn eq(&self, other: &TestUnicode) -> bool {
            self.0 == other.0
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for TestUnicode {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<
                <TestUnicode as crate::__private::PublicFlags>::Internal,
            >;
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for TestUnicode {
        #[inline]
        fn partial_cmp(
            &self,
            other: &TestUnicode,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for TestUnicode {
        #[inline]
        fn cmp(&self, other: &TestUnicode) -> ::core::cmp::Ordering {
            ::core::cmp::Ord::cmp(&self.0, &other.0)
        }
    }
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for TestUnicode {}
    #[automatically_derived]
    impl ::core::clone::Clone for TestUnicode {
        #[inline]
        fn clone(&self) -> TestUnicode {
            let _: ::core::clone::AssertParamIsClone<
                <TestUnicode as crate::__private::PublicFlags>::Internal,
            >;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for TestUnicode {}
    impl TestUnicode {
        /// 1
        #[allow(deprecated, non_upper_case_globals)]
        pub const 一: Self = Self::from_bits_retain(1);
        /// 2
        #[allow(deprecated, non_upper_case_globals)]
        pub const 二: Self = Self::from_bits_retain(1 << 1);
    }
    impl crate::Flags for TestUnicode {
        const FLAGS: &'static [crate::Flag<TestUnicode>] = &[
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("\u{4e00}", TestUnicode::一)
            },
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("\u{4e8c}", TestUnicode::二)
            },
        ];
        type Bits = u8;
        fn bits(&self) -> u8 {
            TestUnicode::bits(self)
        }
        fn from_bits_retain(bits: u8) -> TestUnicode {
            TestUnicode::from_bits_retain(bits)
        }
    }
    #[allow(
        dead_code,
        deprecated,
        unused_doc_comments,
        unused_attributes,
        unused_mut,
        unused_imports,
        non_upper_case_globals,
        clippy::assign_op_pattern,
        clippy::indexing_slicing,
        clippy::same_name_method,
        clippy::iter_without_into_iter,
    )]
    const _: () = {
        #[repr(transparent)]
        pub struct InternalBitFlags(u8);
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::clone::Clone for InternalBitFlags {
            #[inline]
            fn clone(&self) -> InternalBitFlags {
                let _: ::core::clone::AssertParamIsClone<u8>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for InternalBitFlags {
            #[inline]
            fn eq(&self, other: &InternalBitFlags) -> bool {
                self.0 == other.0
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for InternalBitFlags {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<u8>;
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for InternalBitFlags {
            #[inline]
            fn partial_cmp(
                &self,
                other: &InternalBitFlags,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for InternalBitFlags {
            #[inline]
            fn cmp(&self, other: &InternalBitFlags) -> ::core::cmp::Ordering {
                ::core::cmp::Ord::cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for InternalBitFlags {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                ::core::hash::Hash::hash(&self.0, state)
            }
        }
        impl crate::__private::PublicFlags for TestUnicode {
            type Primitive = u8;
            type Internal = InternalBitFlags;
        }
        impl crate::__private::core::default::Default for InternalBitFlags {
            #[inline]
            fn default() -> Self {
                InternalBitFlags::empty()
            }
        }
        impl crate::__private::core::fmt::Debug for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter<'_>,
            ) -> crate::__private::core::fmt::Result {
                if self.is_empty() {
                    f.write_fmt(format_args!("{0:#x}", <u8 as crate::Bits>::EMPTY))
                } else {
                    crate::__private::core::fmt::Display::fmt(self, f)
                }
            }
        }
        impl crate::__private::core::fmt::Display for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter<'_>,
            ) -> crate::__private::core::fmt::Result {
                crate::parser::to_writer(&TestUnicode(*self), f)
            }
        }
        impl crate::__private::core::str::FromStr for InternalBitFlags {
            type Err = crate::parser::ParseError;
            fn from_str(
                s: &str,
            ) -> crate::__private::core::result::Result<Self, Self::Err> {
                crate::parser::from_str::<TestUnicode>(s).map(|flags| flags.0)
            }
        }
        impl crate::__private::core::convert::AsRef<u8> for InternalBitFlags {
            fn as_ref(&self) -> &u8 {
                &self.0
            }
        }
        impl crate::__private::core::convert::From<u8> for InternalBitFlags {
            fn from(bits: u8) -> Self {
                Self::from_bits_retain(bits)
            }
        }
        #[allow(dead_code, deprecated, unused_attributes)]
        impl InternalBitFlags {
            /// Get a flags value with all bits unset.
            #[inline]
            pub const fn empty() -> Self {
                { Self(<u8 as crate::Bits>::EMPTY) }
            }
            /// Get a flags value with all known bits set.
            #[inline]
            pub const fn all() -> Self {
                {
                    let mut truncated = <u8 as crate::Bits>::EMPTY;
                    let mut i = 0;
                    {
                        {
                            let flag = <TestUnicode as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    {
                        {
                            let flag = <TestUnicode as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    let _ = i;
                    Self::from_bits_retain(truncated)
                }
            }
            /// Get the underlying bits value.
            ///
            /// The returned value is exactly the bits set in this flags value.
            #[inline]
            pub const fn bits(&self) -> u8 {
                let f = self;
                { f.0 }
            }
            /// Convert from a bits value.
            ///
            /// This method will return `None` if any unknown bits are set.
            #[inline]
            pub const fn from_bits(
                bits: u8,
            ) -> crate::__private::core::option::Option<Self> {
                let bits = bits;
                {
                    let truncated = Self::from_bits_truncate(bits).0;
                    if truncated == bits {
                        crate::__private::core::option::Option::Some(Self(bits))
                    } else {
                        crate::__private::core::option::Option::None
                    }
                }
            }
            /// Convert from a bits value, unsetting any unknown bits.
            #[inline]
            pub const fn from_bits_truncate(bits: u8) -> Self {
                let bits = bits;
                { Self(bits & Self::all().bits()) }
            }
            /// Convert from a bits value exactly.
            #[inline]
            pub const fn from_bits_retain(bits: u8) -> Self {
                let bits = bits;
                { Self(bits) }
            }
            /// Get a flags value with the bits of a flag with the given name set.
            ///
            /// This method will return `None` if `name` is empty or doesn't
            /// correspond to any named flag.
            #[inline]
            pub fn from_name(
                name: &str,
            ) -> crate::__private::core::option::Option<Self> {
                let name = name;
                {
                    {
                        if name == "\u{4e00}" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestUnicode::一.bits()),
                            );
                        }
                    };
                    {
                        if name == "\u{4e8c}" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestUnicode::二.bits()),
                            );
                        }
                    };
                    let _ = name;
                    crate::__private::core::option::Option::None
                }
            }
            /// Whether all bits in this flags value are unset.
            #[inline]
            pub const fn is_empty(&self) -> bool {
                let f = self;
                { f.bits() == <u8 as crate::Bits>::EMPTY }
            }
            /// Whether all known bits in this flags value are set.
            #[inline]
            pub const fn is_all(&self) -> bool {
                let f = self;
                { Self::all().bits() | f.bits() == f.bits() }
            }
            /// Whether any set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn intersects(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.bits() & other.bits() != <u8 as crate::Bits>::EMPTY }
            }
            /// Whether all set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn contains(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.bits() & other.bits() == other.bits() }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            pub fn insert(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).union(other);
                }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `remove` won't truncate `other`, but the `!` operator will.
            #[inline]
            pub fn remove(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).difference(other);
                }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            pub fn toggle(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).symmetric_difference(other);
                }
            }
            /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
            #[inline]
            pub fn set(&mut self, other: Self, value: bool) {
                let f = self;
                let other = other;
                let value = value;
                {
                    if value {
                        f.insert(other);
                    } else {
                        f.remove(other);
                    }
                }
            }
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() & other.bits()) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn union(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() | other.bits()) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            #[must_use]
            pub const fn difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() & !other.bits()) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn symmetric_difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() ^ other.bits()) }
            }
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            #[must_use]
            pub const fn complement(self) -> Self {
                let f = self;
                { Self::from_bits_truncate(!f.bits()) }
            }
        }
        impl crate::__private::core::fmt::Binary for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Binary::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::Octal for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Octal::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::LowerHex for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::LowerHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::UpperHex for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::UpperHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::ops::BitOr for InternalBitFlags {
            type Output = Self;
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor(self, other: InternalBitFlags) -> Self {
                self.union(other)
            }
        }
        impl crate::__private::core::ops::BitOrAssign for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor_assign(&mut self, other: Self) {
                self.insert(other);
            }
        }
        impl crate::__private::core::ops::BitXor for InternalBitFlags {
            type Output = Self;
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor(self, other: Self) -> Self {
                self.symmetric_difference(other)
            }
        }
        impl crate::__private::core::ops::BitXorAssign for InternalBitFlags {
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor_assign(&mut self, other: Self) {
                self.toggle(other);
            }
        }
        impl crate::__private::core::ops::BitAnd for InternalBitFlags {
            type Output = Self;
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand(self, other: Self) -> Self {
                self.intersection(other)
            }
        }
        impl crate::__private::core::ops::BitAndAssign for InternalBitFlags {
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand_assign(&mut self, other: Self) {
                *self = Self::from_bits_retain(self.bits()).intersection(other);
            }
        }
        impl crate::__private::core::ops::Sub for InternalBitFlags {
            type Output = Self;
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub(self, other: Self) -> Self {
                self.difference(other)
            }
        }
        impl crate::__private::core::ops::SubAssign for InternalBitFlags {
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                self.remove(other);
            }
        }
        impl crate::__private::core::ops::Not for InternalBitFlags {
            type Output = Self;
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            fn not(self) -> Self {
                self.complement()
            }
        }
        impl crate::__private::core::iter::Extend<InternalBitFlags>
        for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn extend<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                &mut self,
                iterator: T,
            ) {
                for item in iterator {
                    self.insert(item)
                }
            }
        }
        impl crate::__private::core::iter::FromIterator<InternalBitFlags>
        for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn from_iter<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                iterator: T,
            ) -> Self {
                use crate::__private::core::iter::Extend;
                let mut result = Self::empty();
                result.extend(iterator);
                result
            }
        }
        impl InternalBitFlags {
            /// Yield a set of contained flags values.
            ///
            /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
            /// will be yielded together as a final flags value.
            #[inline]
            pub const fn iter(&self) -> crate::iter::Iter<TestUnicode> {
                crate::iter::Iter::__private_const_new(
                    <TestUnicode as crate::Flags>::FLAGS,
                    TestUnicode::from_bits_retain(self.bits()),
                    TestUnicode::from_bits_retain(self.bits()),
                )
            }
            /// Yield a set of contained named flags values.
            ///
            /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
            /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
            #[inline]
            pub const fn iter_names(&self) -> crate::iter::IterNames<TestUnicode> {
                crate::iter::IterNames::__private_const_new(
                    <TestUnicode as crate::Flags>::FLAGS,
                    TestUnicode::from_bits_retain(self.bits()),
                    TestUnicode::from_bits_retain(self.bits()),
                )
            }
        }
        impl crate::__private::core::iter::IntoIterator for InternalBitFlags {
            type Item = TestUnicode;
            type IntoIter = crate::iter::Iter<TestUnicode>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
        impl InternalBitFlags {
            /// Returns a mutable reference to the raw value of the flags currently stored.
            #[inline]
            pub fn bits_mut(&mut self) -> &mut u8 {
                &mut self.0
            }
        }
        #[allow(dead_code, deprecated, unused_attributes)]
        impl TestUnicode {
            /// Get a flags value with all bits unset.
            #[inline]
            pub const fn empty() -> Self {
                { Self(InternalBitFlags::empty()) }
            }
            /// Get a flags value with all known bits set.
            #[inline]
            pub const fn all() -> Self {
                { Self(InternalBitFlags::all()) }
            }
            /// Get the underlying bits value.
            ///
            /// The returned value is exactly the bits set in this flags value.
            #[inline]
            pub const fn bits(&self) -> u8 {
                let f = self;
                { f.0.bits() }
            }
            /// Convert from a bits value.
            ///
            /// This method will return `None` if any unknown bits are set.
            #[inline]
            pub const fn from_bits(
                bits: u8,
            ) -> crate::__private::core::option::Option<Self> {
                let bits = bits;
                {
                    match InternalBitFlags::from_bits(bits) {
                        crate::__private::core::option::Option::Some(bits) => {
                            crate::__private::core::option::Option::Some(Self(bits))
                        }
                        crate::__private::core::option::Option::None => {
                            crate::__private::core::option::Option::None
                        }
                    }
                }
            }
            /// Convert from a bits value, unsetting any unknown bits.
            #[inline]
            pub const fn from_bits_truncate(bits: u8) -> Self {
                let bits = bits;
                { Self(InternalBitFlags::from_bits_truncate(bits)) }
            }
            /// Convert from a bits value exactly.
            #[inline]
            pub const fn from_bits_retain(bits: u8) -> Self {
                let bits = bits;
                { Self(InternalBitFlags::from_bits_retain(bits)) }
            }
            /// Get a flags value with the bits of a flag with the given name set.
            ///
            /// This method will return `None` if `name` is empty or doesn't
            /// correspond to any named flag.
            #[inline]
            pub fn from_name(
                name: &str,
            ) -> crate::__private::core::option::Option<Self> {
                let name = name;
                {
                    match InternalBitFlags::from_name(name) {
                        crate::__private::core::option::Option::Some(bits) => {
                            crate::__private::core::option::Option::Some(Self(bits))
                        }
                        crate::__private::core::option::Option::None => {
                            crate::__private::core::option::Option::None
                        }
                    }
                }
            }
            /// Whether all bits in this flags value are unset.
            #[inline]
            pub const fn is_empty(&self) -> bool {
                let f = self;
                { f.0.is_empty() }
            }
            /// Whether all known bits in this flags value are set.
            #[inline]
            pub const fn is_all(&self) -> bool {
                let f = self;
                { f.0.is_all() }
            }
            /// Whether any set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn intersects(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.0.intersects(other.0) }
            }
            /// Whether all set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn contains(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.0.contains(other.0) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            pub fn insert(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.insert(other.0) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `remove` won't truncate `other`, but the `!` operator will.
            #[inline]
            pub fn remove(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.remove(other.0) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            pub fn toggle(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.toggle(other.0) }
            }
            /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
            #[inline]
            pub fn set(&mut self, other: Self, value: bool) {
                let f = self;
                let other = other;
                let value = value;
                { f.0.set(other.0, value) }
            }
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.intersection(other.0)) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn union(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.union(other.0)) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            #[must_use]
            pub const fn difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.difference(other.0)) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn symmetric_difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.symmetric_difference(other.0)) }
            }
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            #[must_use]
            pub const fn complement(self) -> Self {
                let f = self;
                { Self(f.0.complement()) }
            }
        }
        impl crate::__private::core::fmt::Binary for TestUnicode {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Binary::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::Octal for TestUnicode {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Octal::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::LowerHex for TestUnicode {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::LowerHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::UpperHex for TestUnicode {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::UpperHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::ops::BitOr for TestUnicode {
            type Output = Self;
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor(self, other: TestUnicode) -> Self {
                self.union(other)
            }
        }
        impl crate::__private::core::ops::BitOrAssign for TestUnicode {
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor_assign(&mut self, other: Self) {
                self.insert(other);
            }
        }
        impl crate::__private::core::ops::BitXor for TestUnicode {
            type Output = Self;
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor(self, other: Self) -> Self {
                self.symmetric_difference(other)
            }
        }
        impl crate::__private::core::ops::BitXorAssign for TestUnicode {
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor_assign(&mut self, other: Self) {
                self.toggle(other);
            }
        }
        impl crate::__private::core::ops::BitAnd for TestUnicode {
            type Output = Self;
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand(self, other: Self) -> Self {
                self.intersection(other)
            }
        }
        impl crate::__private::core::ops::BitAndAssign for TestUnicode {
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand_assign(&mut self, other: Self) {
                *self = Self::from_bits_retain(self.bits()).intersection(other);
            }
        }
        impl crate::__private::core::ops::Sub for TestUnicode {
            type Output = Self;
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub(self, other: Self) -> Self {
                self.difference(other)
            }
        }
        impl crate::__private::core::ops::SubAssign for TestUnicode {
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                self.remove(other);
            }
        }
        impl crate::__private::core::ops::Not for TestUnicode {
            type Output = Self;
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            fn not(self) -> Self {
                self.complement()
            }
        }
        impl crate::__private::core::iter::Extend<TestUnicode> for TestUnicode {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn extend<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                &mut self,
                iterator: T,
            ) {
                for item in iterator {
                    self.insert(item)
                }
            }
        }
        impl crate::__private::core::iter::FromIterator<TestUnicode> for TestUnicode {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn from_iter<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                iterator: T,
            ) -> Self {
                use crate::__private::core::iter::Extend;
                let mut result = Self::empty();
                result.extend(iterator);
                result
            }
        }
        impl TestUnicode {
            /// Yield a set of contained flags values.
            ///
            /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
            /// will be yielded together as a final flags value.
            #[inline]
            pub const fn iter(&self) -> crate::iter::Iter<TestUnicode> {
                crate::iter::Iter::__private_const_new(
                    <TestUnicode as crate::Flags>::FLAGS,
                    TestUnicode::from_bits_retain(self.bits()),
                    TestUnicode::from_bits_retain(self.bits()),
                )
            }
            /// Yield a set of contained named flags values.
            ///
            /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
            /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
            #[inline]
            pub const fn iter_names(&self) -> crate::iter::IterNames<TestUnicode> {
                crate::iter::IterNames::__private_const_new(
                    <TestUnicode as crate::Flags>::FLAGS,
                    TestUnicode::from_bits_retain(self.bits()),
                    TestUnicode::from_bits_retain(self.bits()),
                )
            }
        }
        impl crate::__private::core::iter::IntoIterator for TestUnicode {
            type Item = TestUnicode;
            type IntoIter = crate::iter::Iter<TestUnicode>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
    };
    pub struct TestEmpty(<TestEmpty as crate::__private::PublicFlags>::Internal);
    #[automatically_derived]
    impl ::core::fmt::Debug for TestEmpty {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_tuple_field1_finish(f, "TestEmpty", &&self.0)
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for TestEmpty {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for TestEmpty {
        #[inline]
        fn eq(&self, other: &TestEmpty) -> bool {
            self.0 == other.0
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for TestEmpty {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<
                <TestEmpty as crate::__private::PublicFlags>::Internal,
            >;
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for TestEmpty {
        #[inline]
        fn partial_cmp(
            &self,
            other: &TestEmpty,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for TestEmpty {
        #[inline]
        fn cmp(&self, other: &TestEmpty) -> ::core::cmp::Ordering {
            ::core::cmp::Ord::cmp(&self.0, &other.0)
        }
    }
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for TestEmpty {}
    #[automatically_derived]
    impl ::core::clone::Clone for TestEmpty {
        #[inline]
        fn clone(&self) -> TestEmpty {
            let _: ::core::clone::AssertParamIsClone<
                <TestEmpty as crate::__private::PublicFlags>::Internal,
            >;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for TestEmpty {}
    impl TestEmpty {}
    impl crate::Flags for TestEmpty {
        const FLAGS: &'static [crate::Flag<TestEmpty>] = &[];
        type Bits = u8;
        fn bits(&self) -> u8 {
            TestEmpty::bits(self)
        }
        fn from_bits_retain(bits: u8) -> TestEmpty {
            TestEmpty::from_bits_retain(bits)
        }
    }
    #[allow(
        dead_code,
        deprecated,
        unused_doc_comments,
        unused_attributes,
        unused_mut,
        unused_imports,
        non_upper_case_globals,
        clippy::assign_op_pattern,
        clippy::indexing_slicing,
        clippy::same_name_method,
        clippy::iter_without_into_iter,
    )]
    const _: () = {
        #[repr(transparent)]
        pub struct InternalBitFlags(u8);
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::clone::Clone for InternalBitFlags {
            #[inline]
            fn clone(&self) -> InternalBitFlags {
                let _: ::core::clone::AssertParamIsClone<u8>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for InternalBitFlags {
            #[inline]
            fn eq(&self, other: &InternalBitFlags) -> bool {
                self.0 == other.0
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for InternalBitFlags {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<u8>;
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for InternalBitFlags {
            #[inline]
            fn partial_cmp(
                &self,
                other: &InternalBitFlags,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for InternalBitFlags {
            #[inline]
            fn cmp(&self, other: &InternalBitFlags) -> ::core::cmp::Ordering {
                ::core::cmp::Ord::cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for InternalBitFlags {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                ::core::hash::Hash::hash(&self.0, state)
            }
        }
        impl crate::__private::PublicFlags for TestEmpty {
            type Primitive = u8;
            type Internal = InternalBitFlags;
        }
        impl crate::__private::core::default::Default for InternalBitFlags {
            #[inline]
            fn default() -> Self {
                InternalBitFlags::empty()
            }
        }
        impl crate::__private::core::fmt::Debug for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter<'_>,
            ) -> crate::__private::core::fmt::Result {
                if self.is_empty() {
                    f.write_fmt(format_args!("{0:#x}", <u8 as crate::Bits>::EMPTY))
                } else {
                    crate::__private::core::fmt::Display::fmt(self, f)
                }
            }
        }
        impl crate::__private::core::fmt::Display for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter<'_>,
            ) -> crate::__private::core::fmt::Result {
                crate::parser::to_writer(&TestEmpty(*self), f)
            }
        }
        impl crate::__private::core::str::FromStr for InternalBitFlags {
            type Err = crate::parser::ParseError;
            fn from_str(
                s: &str,
            ) -> crate::__private::core::result::Result<Self, Self::Err> {
                crate::parser::from_str::<TestEmpty>(s).map(|flags| flags.0)
            }
        }
        impl crate::__private::core::convert::AsRef<u8> for InternalBitFlags {
            fn as_ref(&self) -> &u8 {
                &self.0
            }
        }
        impl crate::__private::core::convert::From<u8> for InternalBitFlags {
            fn from(bits: u8) -> Self {
                Self::from_bits_retain(bits)
            }
        }
        #[allow(dead_code, deprecated, unused_attributes)]
        impl InternalBitFlags {
            /// Get a flags value with all bits unset.
            #[inline]
            pub const fn empty() -> Self {
                { Self(<u8 as crate::Bits>::EMPTY) }
            }
            /// Get a flags value with all known bits set.
            #[inline]
            pub const fn all() -> Self {
                {
                    let mut truncated = <u8 as crate::Bits>::EMPTY;
                    let mut i = 0;
                    let _ = i;
                    Self::from_bits_retain(truncated)
                }
            }
            /// Get the underlying bits value.
            ///
            /// The returned value is exactly the bits set in this flags value.
            #[inline]
            pub const fn bits(&self) -> u8 {
                let f = self;
                { f.0 }
            }
            /// Convert from a bits value.
            ///
            /// This method will return `None` if any unknown bits are set.
            #[inline]
            pub const fn from_bits(
                bits: u8,
            ) -> crate::__private::core::option::Option<Self> {
                let bits = bits;
                {
                    let truncated = Self::from_bits_truncate(bits).0;
                    if truncated == bits {
                        crate::__private::core::option::Option::Some(Self(bits))
                    } else {
                        crate::__private::core::option::Option::None
                    }
                }
            }
            /// Convert from a bits value, unsetting any unknown bits.
            #[inline]
            pub const fn from_bits_truncate(bits: u8) -> Self {
                let bits = bits;
                { Self(bits & Self::all().bits()) }
            }
            /// Convert from a bits value exactly.
            #[inline]
            pub const fn from_bits_retain(bits: u8) -> Self {
                let bits = bits;
                { Self(bits) }
            }
            /// Get a flags value with the bits of a flag with the given name set.
            ///
            /// This method will return `None` if `name` is empty or doesn't
            /// correspond to any named flag.
            #[inline]
            pub fn from_name(
                name: &str,
            ) -> crate::__private::core::option::Option<Self> {
                let name = name;
                {
                    let _ = name;
                    crate::__private::core::option::Option::None
                }
            }
            /// Whether all bits in this flags value are unset.
            #[inline]
            pub const fn is_empty(&self) -> bool {
                let f = self;
                { f.bits() == <u8 as crate::Bits>::EMPTY }
            }
            /// Whether all known bits in this flags value are set.
            #[inline]
            pub const fn is_all(&self) -> bool {
                let f = self;
                { Self::all().bits() | f.bits() == f.bits() }
            }
            /// Whether any set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn intersects(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.bits() & other.bits() != <u8 as crate::Bits>::EMPTY }
            }
            /// Whether all set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn contains(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.bits() & other.bits() == other.bits() }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            pub fn insert(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).union(other);
                }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `remove` won't truncate `other`, but the `!` operator will.
            #[inline]
            pub fn remove(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).difference(other);
                }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            pub fn toggle(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).symmetric_difference(other);
                }
            }
            /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
            #[inline]
            pub fn set(&mut self, other: Self, value: bool) {
                let f = self;
                let other = other;
                let value = value;
                {
                    if value {
                        f.insert(other);
                    } else {
                        f.remove(other);
                    }
                }
            }
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() & other.bits()) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn union(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() | other.bits()) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            #[must_use]
            pub const fn difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() & !other.bits()) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn symmetric_difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() ^ other.bits()) }
            }
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            #[must_use]
            pub const fn complement(self) -> Self {
                let f = self;
                { Self::from_bits_truncate(!f.bits()) }
            }
        }
        impl crate::__private::core::fmt::Binary for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Binary::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::Octal for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Octal::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::LowerHex for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::LowerHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::UpperHex for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::UpperHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::ops::BitOr for InternalBitFlags {
            type Output = Self;
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor(self, other: InternalBitFlags) -> Self {
                self.union(other)
            }
        }
        impl crate::__private::core::ops::BitOrAssign for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor_assign(&mut self, other: Self) {
                self.insert(other);
            }
        }
        impl crate::__private::core::ops::BitXor for InternalBitFlags {
            type Output = Self;
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor(self, other: Self) -> Self {
                self.symmetric_difference(other)
            }
        }
        impl crate::__private::core::ops::BitXorAssign for InternalBitFlags {
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor_assign(&mut self, other: Self) {
                self.toggle(other);
            }
        }
        impl crate::__private::core::ops::BitAnd for InternalBitFlags {
            type Output = Self;
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand(self, other: Self) -> Self {
                self.intersection(other)
            }
        }
        impl crate::__private::core::ops::BitAndAssign for InternalBitFlags {
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand_assign(&mut self, other: Self) {
                *self = Self::from_bits_retain(self.bits()).intersection(other);
            }
        }
        impl crate::__private::core::ops::Sub for InternalBitFlags {
            type Output = Self;
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub(self, other: Self) -> Self {
                self.difference(other)
            }
        }
        impl crate::__private::core::ops::SubAssign for InternalBitFlags {
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                self.remove(other);
            }
        }
        impl crate::__private::core::ops::Not for InternalBitFlags {
            type Output = Self;
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            fn not(self) -> Self {
                self.complement()
            }
        }
        impl crate::__private::core::iter::Extend<InternalBitFlags>
        for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn extend<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                &mut self,
                iterator: T,
            ) {
                for item in iterator {
                    self.insert(item)
                }
            }
        }
        impl crate::__private::core::iter::FromIterator<InternalBitFlags>
        for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn from_iter<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                iterator: T,
            ) -> Self {
                use crate::__private::core::iter::Extend;
                let mut result = Self::empty();
                result.extend(iterator);
                result
            }
        }
        impl InternalBitFlags {
            /// Yield a set of contained flags values.
            ///
            /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
            /// will be yielded together as a final flags value.
            #[inline]
            pub const fn iter(&self) -> crate::iter::Iter<TestEmpty> {
                crate::iter::Iter::__private_const_new(
                    <TestEmpty as crate::Flags>::FLAGS,
                    TestEmpty::from_bits_retain(self.bits()),
                    TestEmpty::from_bits_retain(self.bits()),
                )
            }
            /// Yield a set of contained named flags values.
            ///
            /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
            /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
            #[inline]
            pub const fn iter_names(&self) -> crate::iter::IterNames<TestEmpty> {
                crate::iter::IterNames::__private_const_new(
                    <TestEmpty as crate::Flags>::FLAGS,
                    TestEmpty::from_bits_retain(self.bits()),
                    TestEmpty::from_bits_retain(self.bits()),
                )
            }
        }
        impl crate::__private::core::iter::IntoIterator for InternalBitFlags {
            type Item = TestEmpty;
            type IntoIter = crate::iter::Iter<TestEmpty>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
        impl InternalBitFlags {
            /// Returns a mutable reference to the raw value of the flags currently stored.
            #[inline]
            pub fn bits_mut(&mut self) -> &mut u8 {
                &mut self.0
            }
        }
        #[allow(dead_code, deprecated, unused_attributes)]
        impl TestEmpty {
            /// Get a flags value with all bits unset.
            #[inline]
            pub const fn empty() -> Self {
                { Self(InternalBitFlags::empty()) }
            }
            /// Get a flags value with all known bits set.
            #[inline]
            pub const fn all() -> Self {
                { Self(InternalBitFlags::all()) }
            }
            /// Get the underlying bits value.
            ///
            /// The returned value is exactly the bits set in this flags value.
            #[inline]
            pub const fn bits(&self) -> u8 {
                let f = self;
                { f.0.bits() }
            }
            /// Convert from a bits value.
            ///
            /// This method will return `None` if any unknown bits are set.
            #[inline]
            pub const fn from_bits(
                bits: u8,
            ) -> crate::__private::core::option::Option<Self> {
                let bits = bits;
                {
                    match InternalBitFlags::from_bits(bits) {
                        crate::__private::core::option::Option::Some(bits) => {
                            crate::__private::core::option::Option::Some(Self(bits))
                        }
                        crate::__private::core::option::Option::None => {
                            crate::__private::core::option::Option::None
                        }
                    }
                }
            }
            /// Convert from a bits value, unsetting any unknown bits.
            #[inline]
            pub const fn from_bits_truncate(bits: u8) -> Self {
                let bits = bits;
                { Self(InternalBitFlags::from_bits_truncate(bits)) }
            }
            /// Convert from a bits value exactly.
            #[inline]
            pub const fn from_bits_retain(bits: u8) -> Self {
                let bits = bits;
                { Self(InternalBitFlags::from_bits_retain(bits)) }
            }
            /// Get a flags value with the bits of a flag with the given name set.
            ///
            /// This method will return `None` if `name` is empty or doesn't
            /// correspond to any named flag.
            #[inline]
            pub fn from_name(
                name: &str,
            ) -> crate::__private::core::option::Option<Self> {
                let name = name;
                {
                    match InternalBitFlags::from_name(name) {
                        crate::__private::core::option::Option::Some(bits) => {
                            crate::__private::core::option::Option::Some(Self(bits))
                        }
                        crate::__private::core::option::Option::None => {
                            crate::__private::core::option::Option::None
                        }
                    }
                }
            }
            /// Whether all bits in this flags value are unset.
            #[inline]
            pub const fn is_empty(&self) -> bool {
                let f = self;
                { f.0.is_empty() }
            }
            /// Whether all known bits in this flags value are set.
            #[inline]
            pub const fn is_all(&self) -> bool {
                let f = self;
                { f.0.is_all() }
            }
            /// Whether any set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn intersects(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.0.intersects(other.0) }
            }
            /// Whether all set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn contains(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.0.contains(other.0) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            pub fn insert(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.insert(other.0) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `remove` won't truncate `other`, but the `!` operator will.
            #[inline]
            pub fn remove(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.remove(other.0) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            pub fn toggle(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.toggle(other.0) }
            }
            /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
            #[inline]
            pub fn set(&mut self, other: Self, value: bool) {
                let f = self;
                let other = other;
                let value = value;
                { f.0.set(other.0, value) }
            }
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.intersection(other.0)) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn union(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.union(other.0)) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            #[must_use]
            pub const fn difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.difference(other.0)) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn symmetric_difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.symmetric_difference(other.0)) }
            }
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            #[must_use]
            pub const fn complement(self) -> Self {
                let f = self;
                { Self(f.0.complement()) }
            }
        }
        impl crate::__private::core::fmt::Binary for TestEmpty {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Binary::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::Octal for TestEmpty {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Octal::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::LowerHex for TestEmpty {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::LowerHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::UpperHex for TestEmpty {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::UpperHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::ops::BitOr for TestEmpty {
            type Output = Self;
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor(self, other: TestEmpty) -> Self {
                self.union(other)
            }
        }
        impl crate::__private::core::ops::BitOrAssign for TestEmpty {
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor_assign(&mut self, other: Self) {
                self.insert(other);
            }
        }
        impl crate::__private::core::ops::BitXor for TestEmpty {
            type Output = Self;
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor(self, other: Self) -> Self {
                self.symmetric_difference(other)
            }
        }
        impl crate::__private::core::ops::BitXorAssign for TestEmpty {
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor_assign(&mut self, other: Self) {
                self.toggle(other);
            }
        }
        impl crate::__private::core::ops::BitAnd for TestEmpty {
            type Output = Self;
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand(self, other: Self) -> Self {
                self.intersection(other)
            }
        }
        impl crate::__private::core::ops::BitAndAssign for TestEmpty {
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand_assign(&mut self, other: Self) {
                *self = Self::from_bits_retain(self.bits()).intersection(other);
            }
        }
        impl crate::__private::core::ops::Sub for TestEmpty {
            type Output = Self;
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub(self, other: Self) -> Self {
                self.difference(other)
            }
        }
        impl crate::__private::core::ops::SubAssign for TestEmpty {
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                self.remove(other);
            }
        }
        impl crate::__private::core::ops::Not for TestEmpty {
            type Output = Self;
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            fn not(self) -> Self {
                self.complement()
            }
        }
        impl crate::__private::core::iter::Extend<TestEmpty> for TestEmpty {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn extend<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                &mut self,
                iterator: T,
            ) {
                for item in iterator {
                    self.insert(item)
                }
            }
        }
        impl crate::__private::core::iter::FromIterator<TestEmpty> for TestEmpty {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn from_iter<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                iterator: T,
            ) -> Self {
                use crate::__private::core::iter::Extend;
                let mut result = Self::empty();
                result.extend(iterator);
                result
            }
        }
        impl TestEmpty {
            /// Yield a set of contained flags values.
            ///
            /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
            /// will be yielded together as a final flags value.
            #[inline]
            pub const fn iter(&self) -> crate::iter::Iter<TestEmpty> {
                crate::iter::Iter::__private_const_new(
                    <TestEmpty as crate::Flags>::FLAGS,
                    TestEmpty::from_bits_retain(self.bits()),
                    TestEmpty::from_bits_retain(self.bits()),
                )
            }
            /// Yield a set of contained named flags values.
            ///
            /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
            /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
            #[inline]
            pub const fn iter_names(&self) -> crate::iter::IterNames<TestEmpty> {
                crate::iter::IterNames::__private_const_new(
                    <TestEmpty as crate::Flags>::FLAGS,
                    TestEmpty::from_bits_retain(self.bits()),
                    TestEmpty::from_bits_retain(self.bits()),
                )
            }
        }
        impl crate::__private::core::iter::IntoIterator for TestEmpty {
            type Item = TestEmpty;
            type IntoIter = crate::iter::Iter<TestEmpty>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
    };
    pub struct TestOverlapping(
        <TestOverlapping as crate::__private::PublicFlags>::Internal,
    );
    #[automatically_derived]
    impl ::core::fmt::Debug for TestOverlapping {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_tuple_field1_finish(
                f,
                "TestOverlapping",
                &&self.0,
            )
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for TestOverlapping {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for TestOverlapping {
        #[inline]
        fn eq(&self, other: &TestOverlapping) -> bool {
            self.0 == other.0
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for TestOverlapping {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<
                <TestOverlapping as crate::__private::PublicFlags>::Internal,
            >;
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for TestOverlapping {
        #[inline]
        fn partial_cmp(
            &self,
            other: &TestOverlapping,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for TestOverlapping {
        #[inline]
        fn cmp(&self, other: &TestOverlapping) -> ::core::cmp::Ordering {
            ::core::cmp::Ord::cmp(&self.0, &other.0)
        }
    }
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for TestOverlapping {}
    #[automatically_derived]
    impl ::core::clone::Clone for TestOverlapping {
        #[inline]
        fn clone(&self) -> TestOverlapping {
            let _: ::core::clone::AssertParamIsClone<
                <TestOverlapping as crate::__private::PublicFlags>::Internal,
            >;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for TestOverlapping {}
    impl TestOverlapping {
        /// 1 | (1 << 1)
        #[allow(deprecated, non_upper_case_globals)]
        pub const AB: Self = Self::from_bits_retain(1 | (1 << 1));
        /// (1 << 1) | (1 << 2)
        #[allow(deprecated, non_upper_case_globals)]
        pub const BC: Self = Self::from_bits_retain((1 << 1) | (1 << 2));
    }
    impl crate::Flags for TestOverlapping {
        const FLAGS: &'static [crate::Flag<TestOverlapping>] = &[
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("AB", TestOverlapping::AB)
            },
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("BC", TestOverlapping::BC)
            },
        ];
        type Bits = u8;
        fn bits(&self) -> u8 {
            TestOverlapping::bits(self)
        }
        fn from_bits_retain(bits: u8) -> TestOverlapping {
            TestOverlapping::from_bits_retain(bits)
        }
    }
    #[allow(
        dead_code,
        deprecated,
        unused_doc_comments,
        unused_attributes,
        unused_mut,
        unused_imports,
        non_upper_case_globals,
        clippy::assign_op_pattern,
        clippy::indexing_slicing,
        clippy::same_name_method,
        clippy::iter_without_into_iter,
    )]
    const _: () = {
        #[repr(transparent)]
        pub struct InternalBitFlags(u8);
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::clone::Clone for InternalBitFlags {
            #[inline]
            fn clone(&self) -> InternalBitFlags {
                let _: ::core::clone::AssertParamIsClone<u8>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for InternalBitFlags {
            #[inline]
            fn eq(&self, other: &InternalBitFlags) -> bool {
                self.0 == other.0
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for InternalBitFlags {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<u8>;
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for InternalBitFlags {
            #[inline]
            fn partial_cmp(
                &self,
                other: &InternalBitFlags,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for InternalBitFlags {
            #[inline]
            fn cmp(&self, other: &InternalBitFlags) -> ::core::cmp::Ordering {
                ::core::cmp::Ord::cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for InternalBitFlags {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                ::core::hash::Hash::hash(&self.0, state)
            }
        }
        impl crate::__private::PublicFlags for TestOverlapping {
            type Primitive = u8;
            type Internal = InternalBitFlags;
        }
        impl crate::__private::core::default::Default for InternalBitFlags {
            #[inline]
            fn default() -> Self {
                InternalBitFlags::empty()
            }
        }
        impl crate::__private::core::fmt::Debug for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter<'_>,
            ) -> crate::__private::core::fmt::Result {
                if self.is_empty() {
                    f.write_fmt(format_args!("{0:#x}", <u8 as crate::Bits>::EMPTY))
                } else {
                    crate::__private::core::fmt::Display::fmt(self, f)
                }
            }
        }
        impl crate::__private::core::fmt::Display for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter<'_>,
            ) -> crate::__private::core::fmt::Result {
                crate::parser::to_writer(&TestOverlapping(*self), f)
            }
        }
        impl crate::__private::core::str::FromStr for InternalBitFlags {
            type Err = crate::parser::ParseError;
            fn from_str(
                s: &str,
            ) -> crate::__private::core::result::Result<Self, Self::Err> {
                crate::parser::from_str::<TestOverlapping>(s).map(|flags| flags.0)
            }
        }
        impl crate::__private::core::convert::AsRef<u8> for InternalBitFlags {
            fn as_ref(&self) -> &u8 {
                &self.0
            }
        }
        impl crate::__private::core::convert::From<u8> for InternalBitFlags {
            fn from(bits: u8) -> Self {
                Self::from_bits_retain(bits)
            }
        }
        #[allow(dead_code, deprecated, unused_attributes)]
        impl InternalBitFlags {
            /// Get a flags value with all bits unset.
            #[inline]
            pub const fn empty() -> Self {
                { Self(<u8 as crate::Bits>::EMPTY) }
            }
            /// Get a flags value with all known bits set.
            #[inline]
            pub const fn all() -> Self {
                {
                    let mut truncated = <u8 as crate::Bits>::EMPTY;
                    let mut i = 0;
                    {
                        {
                            let flag = <TestOverlapping as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    {
                        {
                            let flag = <TestOverlapping as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    let _ = i;
                    Self::from_bits_retain(truncated)
                }
            }
            /// Get the underlying bits value.
            ///
            /// The returned value is exactly the bits set in this flags value.
            #[inline]
            pub const fn bits(&self) -> u8 {
                let f = self;
                { f.0 }
            }
            /// Convert from a bits value.
            ///
            /// This method will return `None` if any unknown bits are set.
            #[inline]
            pub const fn from_bits(
                bits: u8,
            ) -> crate::__private::core::option::Option<Self> {
                let bits = bits;
                {
                    let truncated = Self::from_bits_truncate(bits).0;
                    if truncated == bits {
                        crate::__private::core::option::Option::Some(Self(bits))
                    } else {
                        crate::__private::core::option::Option::None
                    }
                }
            }
            /// Convert from a bits value, unsetting any unknown bits.
            #[inline]
            pub const fn from_bits_truncate(bits: u8) -> Self {
                let bits = bits;
                { Self(bits & Self::all().bits()) }
            }
            /// Convert from a bits value exactly.
            #[inline]
            pub const fn from_bits_retain(bits: u8) -> Self {
                let bits = bits;
                { Self(bits) }
            }
            /// Get a flags value with the bits of a flag with the given name set.
            ///
            /// This method will return `None` if `name` is empty or doesn't
            /// correspond to any named flag.
            #[inline]
            pub fn from_name(
                name: &str,
            ) -> crate::__private::core::option::Option<Self> {
                let name = name;
                {
                    {
                        if name == "AB" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestOverlapping::AB.bits()),
                            );
                        }
                    };
                    {
                        if name == "BC" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestOverlapping::BC.bits()),
                            );
                        }
                    };
                    let _ = name;
                    crate::__private::core::option::Option::None
                }
            }
            /// Whether all bits in this flags value are unset.
            #[inline]
            pub const fn is_empty(&self) -> bool {
                let f = self;
                { f.bits() == <u8 as crate::Bits>::EMPTY }
            }
            /// Whether all known bits in this flags value are set.
            #[inline]
            pub const fn is_all(&self) -> bool {
                let f = self;
                { Self::all().bits() | f.bits() == f.bits() }
            }
            /// Whether any set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn intersects(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.bits() & other.bits() != <u8 as crate::Bits>::EMPTY }
            }
            /// Whether all set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn contains(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.bits() & other.bits() == other.bits() }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            pub fn insert(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).union(other);
                }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `remove` won't truncate `other`, but the `!` operator will.
            #[inline]
            pub fn remove(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).difference(other);
                }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            pub fn toggle(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).symmetric_difference(other);
                }
            }
            /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
            #[inline]
            pub fn set(&mut self, other: Self, value: bool) {
                let f = self;
                let other = other;
                let value = value;
                {
                    if value {
                        f.insert(other);
                    } else {
                        f.remove(other);
                    }
                }
            }
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() & other.bits()) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn union(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() | other.bits()) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            #[must_use]
            pub const fn difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() & !other.bits()) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn symmetric_difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() ^ other.bits()) }
            }
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            #[must_use]
            pub const fn complement(self) -> Self {
                let f = self;
                { Self::from_bits_truncate(!f.bits()) }
            }
        }
        impl crate::__private::core::fmt::Binary for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Binary::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::Octal for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Octal::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::LowerHex for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::LowerHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::UpperHex for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::UpperHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::ops::BitOr for InternalBitFlags {
            type Output = Self;
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor(self, other: InternalBitFlags) -> Self {
                self.union(other)
            }
        }
        impl crate::__private::core::ops::BitOrAssign for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor_assign(&mut self, other: Self) {
                self.insert(other);
            }
        }
        impl crate::__private::core::ops::BitXor for InternalBitFlags {
            type Output = Self;
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor(self, other: Self) -> Self {
                self.symmetric_difference(other)
            }
        }
        impl crate::__private::core::ops::BitXorAssign for InternalBitFlags {
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor_assign(&mut self, other: Self) {
                self.toggle(other);
            }
        }
        impl crate::__private::core::ops::BitAnd for InternalBitFlags {
            type Output = Self;
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand(self, other: Self) -> Self {
                self.intersection(other)
            }
        }
        impl crate::__private::core::ops::BitAndAssign for InternalBitFlags {
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand_assign(&mut self, other: Self) {
                *self = Self::from_bits_retain(self.bits()).intersection(other);
            }
        }
        impl crate::__private::core::ops::Sub for InternalBitFlags {
            type Output = Self;
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub(self, other: Self) -> Self {
                self.difference(other)
            }
        }
        impl crate::__private::core::ops::SubAssign for InternalBitFlags {
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                self.remove(other);
            }
        }
        impl crate::__private::core::ops::Not for InternalBitFlags {
            type Output = Self;
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            fn not(self) -> Self {
                self.complement()
            }
        }
        impl crate::__private::core::iter::Extend<InternalBitFlags>
        for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn extend<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                &mut self,
                iterator: T,
            ) {
                for item in iterator {
                    self.insert(item)
                }
            }
        }
        impl crate::__private::core::iter::FromIterator<InternalBitFlags>
        for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn from_iter<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                iterator: T,
            ) -> Self {
                use crate::__private::core::iter::Extend;
                let mut result = Self::empty();
                result.extend(iterator);
                result
            }
        }
        impl InternalBitFlags {
            /// Yield a set of contained flags values.
            ///
            /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
            /// will be yielded together as a final flags value.
            #[inline]
            pub const fn iter(&self) -> crate::iter::Iter<TestOverlapping> {
                crate::iter::Iter::__private_const_new(
                    <TestOverlapping as crate::Flags>::FLAGS,
                    TestOverlapping::from_bits_retain(self.bits()),
                    TestOverlapping::from_bits_retain(self.bits()),
                )
            }
            /// Yield a set of contained named flags values.
            ///
            /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
            /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
            #[inline]
            pub const fn iter_names(&self) -> crate::iter::IterNames<TestOverlapping> {
                crate::iter::IterNames::__private_const_new(
                    <TestOverlapping as crate::Flags>::FLAGS,
                    TestOverlapping::from_bits_retain(self.bits()),
                    TestOverlapping::from_bits_retain(self.bits()),
                )
            }
        }
        impl crate::__private::core::iter::IntoIterator for InternalBitFlags {
            type Item = TestOverlapping;
            type IntoIter = crate::iter::Iter<TestOverlapping>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
        impl InternalBitFlags {
            /// Returns a mutable reference to the raw value of the flags currently stored.
            #[inline]
            pub fn bits_mut(&mut self) -> &mut u8 {
                &mut self.0
            }
        }
        #[allow(dead_code, deprecated, unused_attributes)]
        impl TestOverlapping {
            /// Get a flags value with all bits unset.
            #[inline]
            pub const fn empty() -> Self {
                { Self(InternalBitFlags::empty()) }
            }
            /// Get a flags value with all known bits set.
            #[inline]
            pub const fn all() -> Self {
                { Self(InternalBitFlags::all()) }
            }
            /// Get the underlying bits value.
            ///
            /// The returned value is exactly the bits set in this flags value.
            #[inline]
            pub const fn bits(&self) -> u8 {
                let f = self;
                { f.0.bits() }
            }
            /// Convert from a bits value.
            ///
            /// This method will return `None` if any unknown bits are set.
            #[inline]
            pub const fn from_bits(
                bits: u8,
            ) -> crate::__private::core::option::Option<Self> {
                let bits = bits;
                {
                    match InternalBitFlags::from_bits(bits) {
                        crate::__private::core::option::Option::Some(bits) => {
                            crate::__private::core::option::Option::Some(Self(bits))
                        }
                        crate::__private::core::option::Option::None => {
                            crate::__private::core::option::Option::None
                        }
                    }
                }
            }
            /// Convert from a bits value, unsetting any unknown bits.
            #[inline]
            pub const fn from_bits_truncate(bits: u8) -> Self {
                let bits = bits;
                { Self(InternalBitFlags::from_bits_truncate(bits)) }
            }
            /// Convert from a bits value exactly.
            #[inline]
            pub const fn from_bits_retain(bits: u8) -> Self {
                let bits = bits;
                { Self(InternalBitFlags::from_bits_retain(bits)) }
            }
            /// Get a flags value with the bits of a flag with the given name set.
            ///
            /// This method will return `None` if `name` is empty or doesn't
            /// correspond to any named flag.
            #[inline]
            pub fn from_name(
                name: &str,
            ) -> crate::__private::core::option::Option<Self> {
                let name = name;
                {
                    match InternalBitFlags::from_name(name) {
                        crate::__private::core::option::Option::Some(bits) => {
                            crate::__private::core::option::Option::Some(Self(bits))
                        }
                        crate::__private::core::option::Option::None => {
                            crate::__private::core::option::Option::None
                        }
                    }
                }
            }
            /// Whether all bits in this flags value are unset.
            #[inline]
            pub const fn is_empty(&self) -> bool {
                let f = self;
                { f.0.is_empty() }
            }
            /// Whether all known bits in this flags value are set.
            #[inline]
            pub const fn is_all(&self) -> bool {
                let f = self;
                { f.0.is_all() }
            }
            /// Whether any set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn intersects(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.0.intersects(other.0) }
            }
            /// Whether all set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn contains(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.0.contains(other.0) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            pub fn insert(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.insert(other.0) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `remove` won't truncate `other`, but the `!` operator will.
            #[inline]
            pub fn remove(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.remove(other.0) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            pub fn toggle(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.toggle(other.0) }
            }
            /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
            #[inline]
            pub fn set(&mut self, other: Self, value: bool) {
                let f = self;
                let other = other;
                let value = value;
                { f.0.set(other.0, value) }
            }
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.intersection(other.0)) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn union(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.union(other.0)) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            #[must_use]
            pub const fn difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.difference(other.0)) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn symmetric_difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.symmetric_difference(other.0)) }
            }
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            #[must_use]
            pub const fn complement(self) -> Self {
                let f = self;
                { Self(f.0.complement()) }
            }
        }
        impl crate::__private::core::fmt::Binary for TestOverlapping {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Binary::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::Octal for TestOverlapping {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Octal::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::LowerHex for TestOverlapping {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::LowerHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::UpperHex for TestOverlapping {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::UpperHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::ops::BitOr for TestOverlapping {
            type Output = Self;
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor(self, other: TestOverlapping) -> Self {
                self.union(other)
            }
        }
        impl crate::__private::core::ops::BitOrAssign for TestOverlapping {
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor_assign(&mut self, other: Self) {
                self.insert(other);
            }
        }
        impl crate::__private::core::ops::BitXor for TestOverlapping {
            type Output = Self;
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor(self, other: Self) -> Self {
                self.symmetric_difference(other)
            }
        }
        impl crate::__private::core::ops::BitXorAssign for TestOverlapping {
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor_assign(&mut self, other: Self) {
                self.toggle(other);
            }
        }
        impl crate::__private::core::ops::BitAnd for TestOverlapping {
            type Output = Self;
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand(self, other: Self) -> Self {
                self.intersection(other)
            }
        }
        impl crate::__private::core::ops::BitAndAssign for TestOverlapping {
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand_assign(&mut self, other: Self) {
                *self = Self::from_bits_retain(self.bits()).intersection(other);
            }
        }
        impl crate::__private::core::ops::Sub for TestOverlapping {
            type Output = Self;
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub(self, other: Self) -> Self {
                self.difference(other)
            }
        }
        impl crate::__private::core::ops::SubAssign for TestOverlapping {
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                self.remove(other);
            }
        }
        impl crate::__private::core::ops::Not for TestOverlapping {
            type Output = Self;
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            fn not(self) -> Self {
                self.complement()
            }
        }
        impl crate::__private::core::iter::Extend<TestOverlapping> for TestOverlapping {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn extend<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                &mut self,
                iterator: T,
            ) {
                for item in iterator {
                    self.insert(item)
                }
            }
        }
        impl crate::__private::core::iter::FromIterator<TestOverlapping>
        for TestOverlapping {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn from_iter<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                iterator: T,
            ) -> Self {
                use crate::__private::core::iter::Extend;
                let mut result = Self::empty();
                result.extend(iterator);
                result
            }
        }
        impl TestOverlapping {
            /// Yield a set of contained flags values.
            ///
            /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
            /// will be yielded together as a final flags value.
            #[inline]
            pub const fn iter(&self) -> crate::iter::Iter<TestOverlapping> {
                crate::iter::Iter::__private_const_new(
                    <TestOverlapping as crate::Flags>::FLAGS,
                    TestOverlapping::from_bits_retain(self.bits()),
                    TestOverlapping::from_bits_retain(self.bits()),
                )
            }
            /// Yield a set of contained named flags values.
            ///
            /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
            /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
            #[inline]
            pub const fn iter_names(&self) -> crate::iter::IterNames<TestOverlapping> {
                crate::iter::IterNames::__private_const_new(
                    <TestOverlapping as crate::Flags>::FLAGS,
                    TestOverlapping::from_bits_retain(self.bits()),
                    TestOverlapping::from_bits_retain(self.bits()),
                )
            }
        }
        impl crate::__private::core::iter::IntoIterator for TestOverlapping {
            type Item = TestOverlapping;
            type IntoIter = crate::iter::Iter<TestOverlapping>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
    };
    pub struct TestOverlappingFull(
        <TestOverlappingFull as crate::__private::PublicFlags>::Internal,
    );
    #[automatically_derived]
    impl ::core::fmt::Debug for TestOverlappingFull {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_tuple_field1_finish(
                f,
                "TestOverlappingFull",
                &&self.0,
            )
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for TestOverlappingFull {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for TestOverlappingFull {
        #[inline]
        fn eq(&self, other: &TestOverlappingFull) -> bool {
            self.0 == other.0
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for TestOverlappingFull {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<
                <TestOverlappingFull as crate::__private::PublicFlags>::Internal,
            >;
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for TestOverlappingFull {
        #[inline]
        fn partial_cmp(
            &self,
            other: &TestOverlappingFull,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for TestOverlappingFull {
        #[inline]
        fn cmp(&self, other: &TestOverlappingFull) -> ::core::cmp::Ordering {
            ::core::cmp::Ord::cmp(&self.0, &other.0)
        }
    }
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for TestOverlappingFull {}
    #[automatically_derived]
    impl ::core::clone::Clone for TestOverlappingFull {
        #[inline]
        fn clone(&self) -> TestOverlappingFull {
            let _: ::core::clone::AssertParamIsClone<
                <TestOverlappingFull as crate::__private::PublicFlags>::Internal,
            >;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for TestOverlappingFull {}
    impl TestOverlappingFull {
        /// 1
        #[allow(deprecated, non_upper_case_globals)]
        pub const A: Self = Self::from_bits_retain(1);
        /// 1
        #[allow(deprecated, non_upper_case_globals)]
        pub const B: Self = Self::from_bits_retain(1);
        /// 1
        #[allow(deprecated, non_upper_case_globals)]
        pub const C: Self = Self::from_bits_retain(1);
        /// 2
        #[allow(deprecated, non_upper_case_globals)]
        pub const D: Self = Self::from_bits_retain(1 << 1);
    }
    impl crate::Flags for TestOverlappingFull {
        const FLAGS: &'static [crate::Flag<TestOverlappingFull>] = &[
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("A", TestOverlappingFull::A)
            },
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("B", TestOverlappingFull::B)
            },
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("C", TestOverlappingFull::C)
            },
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("D", TestOverlappingFull::D)
            },
        ];
        type Bits = u8;
        fn bits(&self) -> u8 {
            TestOverlappingFull::bits(self)
        }
        fn from_bits_retain(bits: u8) -> TestOverlappingFull {
            TestOverlappingFull::from_bits_retain(bits)
        }
    }
    #[allow(
        dead_code,
        deprecated,
        unused_doc_comments,
        unused_attributes,
        unused_mut,
        unused_imports,
        non_upper_case_globals,
        clippy::assign_op_pattern,
        clippy::indexing_slicing,
        clippy::same_name_method,
        clippy::iter_without_into_iter,
    )]
    const _: () = {
        #[repr(transparent)]
        pub struct InternalBitFlags(u8);
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::clone::Clone for InternalBitFlags {
            #[inline]
            fn clone(&self) -> InternalBitFlags {
                let _: ::core::clone::AssertParamIsClone<u8>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for InternalBitFlags {
            #[inline]
            fn eq(&self, other: &InternalBitFlags) -> bool {
                self.0 == other.0
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for InternalBitFlags {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<u8>;
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for InternalBitFlags {
            #[inline]
            fn partial_cmp(
                &self,
                other: &InternalBitFlags,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for InternalBitFlags {
            #[inline]
            fn cmp(&self, other: &InternalBitFlags) -> ::core::cmp::Ordering {
                ::core::cmp::Ord::cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for InternalBitFlags {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                ::core::hash::Hash::hash(&self.0, state)
            }
        }
        impl crate::__private::PublicFlags for TestOverlappingFull {
            type Primitive = u8;
            type Internal = InternalBitFlags;
        }
        impl crate::__private::core::default::Default for InternalBitFlags {
            #[inline]
            fn default() -> Self {
                InternalBitFlags::empty()
            }
        }
        impl crate::__private::core::fmt::Debug for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter<'_>,
            ) -> crate::__private::core::fmt::Result {
                if self.is_empty() {
                    f.write_fmt(format_args!("{0:#x}", <u8 as crate::Bits>::EMPTY))
                } else {
                    crate::__private::core::fmt::Display::fmt(self, f)
                }
            }
        }
        impl crate::__private::core::fmt::Display for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter<'_>,
            ) -> crate::__private::core::fmt::Result {
                crate::parser::to_writer(&TestOverlappingFull(*self), f)
            }
        }
        impl crate::__private::core::str::FromStr for InternalBitFlags {
            type Err = crate::parser::ParseError;
            fn from_str(
                s: &str,
            ) -> crate::__private::core::result::Result<Self, Self::Err> {
                crate::parser::from_str::<TestOverlappingFull>(s).map(|flags| flags.0)
            }
        }
        impl crate::__private::core::convert::AsRef<u8> for InternalBitFlags {
            fn as_ref(&self) -> &u8 {
                &self.0
            }
        }
        impl crate::__private::core::convert::From<u8> for InternalBitFlags {
            fn from(bits: u8) -> Self {
                Self::from_bits_retain(bits)
            }
        }
        #[allow(dead_code, deprecated, unused_attributes)]
        impl InternalBitFlags {
            /// Get a flags value with all bits unset.
            #[inline]
            pub const fn empty() -> Self {
                { Self(<u8 as crate::Bits>::EMPTY) }
            }
            /// Get a flags value with all known bits set.
            #[inline]
            pub const fn all() -> Self {
                {
                    let mut truncated = <u8 as crate::Bits>::EMPTY;
                    let mut i = 0;
                    {
                        {
                            let flag = <TestOverlappingFull as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    {
                        {
                            let flag = <TestOverlappingFull as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    {
                        {
                            let flag = <TestOverlappingFull as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    {
                        {
                            let flag = <TestOverlappingFull as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    let _ = i;
                    Self::from_bits_retain(truncated)
                }
            }
            /// Get the underlying bits value.
            ///
            /// The returned value is exactly the bits set in this flags value.
            #[inline]
            pub const fn bits(&self) -> u8 {
                let f = self;
                { f.0 }
            }
            /// Convert from a bits value.
            ///
            /// This method will return `None` if any unknown bits are set.
            #[inline]
            pub const fn from_bits(
                bits: u8,
            ) -> crate::__private::core::option::Option<Self> {
                let bits = bits;
                {
                    let truncated = Self::from_bits_truncate(bits).0;
                    if truncated == bits {
                        crate::__private::core::option::Option::Some(Self(bits))
                    } else {
                        crate::__private::core::option::Option::None
                    }
                }
            }
            /// Convert from a bits value, unsetting any unknown bits.
            #[inline]
            pub const fn from_bits_truncate(bits: u8) -> Self {
                let bits = bits;
                { Self(bits & Self::all().bits()) }
            }
            /// Convert from a bits value exactly.
            #[inline]
            pub const fn from_bits_retain(bits: u8) -> Self {
                let bits = bits;
                { Self(bits) }
            }
            /// Get a flags value with the bits of a flag with the given name set.
            ///
            /// This method will return `None` if `name` is empty or doesn't
            /// correspond to any named flag.
            #[inline]
            pub fn from_name(
                name: &str,
            ) -> crate::__private::core::option::Option<Self> {
                let name = name;
                {
                    {
                        if name == "A" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestOverlappingFull::A.bits()),
                            );
                        }
                    };
                    {
                        if name == "B" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestOverlappingFull::B.bits()),
                            );
                        }
                    };
                    {
                        if name == "C" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestOverlappingFull::C.bits()),
                            );
                        }
                    };
                    {
                        if name == "D" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestOverlappingFull::D.bits()),
                            );
                        }
                    };
                    let _ = name;
                    crate::__private::core::option::Option::None
                }
            }
            /// Whether all bits in this flags value are unset.
            #[inline]
            pub const fn is_empty(&self) -> bool {
                let f = self;
                { f.bits() == <u8 as crate::Bits>::EMPTY }
            }
            /// Whether all known bits in this flags value are set.
            #[inline]
            pub const fn is_all(&self) -> bool {
                let f = self;
                { Self::all().bits() | f.bits() == f.bits() }
            }
            /// Whether any set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn intersects(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.bits() & other.bits() != <u8 as crate::Bits>::EMPTY }
            }
            /// Whether all set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn contains(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.bits() & other.bits() == other.bits() }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            pub fn insert(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).union(other);
                }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `remove` won't truncate `other`, but the `!` operator will.
            #[inline]
            pub fn remove(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).difference(other);
                }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            pub fn toggle(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).symmetric_difference(other);
                }
            }
            /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
            #[inline]
            pub fn set(&mut self, other: Self, value: bool) {
                let f = self;
                let other = other;
                let value = value;
                {
                    if value {
                        f.insert(other);
                    } else {
                        f.remove(other);
                    }
                }
            }
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() & other.bits()) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn union(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() | other.bits()) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            #[must_use]
            pub const fn difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() & !other.bits()) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn symmetric_difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() ^ other.bits()) }
            }
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            #[must_use]
            pub const fn complement(self) -> Self {
                let f = self;
                { Self::from_bits_truncate(!f.bits()) }
            }
        }
        impl crate::__private::core::fmt::Binary for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Binary::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::Octal for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Octal::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::LowerHex for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::LowerHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::UpperHex for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::UpperHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::ops::BitOr for InternalBitFlags {
            type Output = Self;
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor(self, other: InternalBitFlags) -> Self {
                self.union(other)
            }
        }
        impl crate::__private::core::ops::BitOrAssign for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor_assign(&mut self, other: Self) {
                self.insert(other);
            }
        }
        impl crate::__private::core::ops::BitXor for InternalBitFlags {
            type Output = Self;
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor(self, other: Self) -> Self {
                self.symmetric_difference(other)
            }
        }
        impl crate::__private::core::ops::BitXorAssign for InternalBitFlags {
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor_assign(&mut self, other: Self) {
                self.toggle(other);
            }
        }
        impl crate::__private::core::ops::BitAnd for InternalBitFlags {
            type Output = Self;
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand(self, other: Self) -> Self {
                self.intersection(other)
            }
        }
        impl crate::__private::core::ops::BitAndAssign for InternalBitFlags {
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand_assign(&mut self, other: Self) {
                *self = Self::from_bits_retain(self.bits()).intersection(other);
            }
        }
        impl crate::__private::core::ops::Sub for InternalBitFlags {
            type Output = Self;
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub(self, other: Self) -> Self {
                self.difference(other)
            }
        }
        impl crate::__private::core::ops::SubAssign for InternalBitFlags {
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                self.remove(other);
            }
        }
        impl crate::__private::core::ops::Not for InternalBitFlags {
            type Output = Self;
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            fn not(self) -> Self {
                self.complement()
            }
        }
        impl crate::__private::core::iter::Extend<InternalBitFlags>
        for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn extend<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                &mut self,
                iterator: T,
            ) {
                for item in iterator {
                    self.insert(item)
                }
            }
        }
        impl crate::__private::core::iter::FromIterator<InternalBitFlags>
        for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn from_iter<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                iterator: T,
            ) -> Self {
                use crate::__private::core::iter::Extend;
                let mut result = Self::empty();
                result.extend(iterator);
                result
            }
        }
        impl InternalBitFlags {
            /// Yield a set of contained flags values.
            ///
            /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
            /// will be yielded together as a final flags value.
            #[inline]
            pub const fn iter(&self) -> crate::iter::Iter<TestOverlappingFull> {
                crate::iter::Iter::__private_const_new(
                    <TestOverlappingFull as crate::Flags>::FLAGS,
                    TestOverlappingFull::from_bits_retain(self.bits()),
                    TestOverlappingFull::from_bits_retain(self.bits()),
                )
            }
            /// Yield a set of contained named flags values.
            ///
            /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
            /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
            #[inline]
            pub const fn iter_names(
                &self,
            ) -> crate::iter::IterNames<TestOverlappingFull> {
                crate::iter::IterNames::__private_const_new(
                    <TestOverlappingFull as crate::Flags>::FLAGS,
                    TestOverlappingFull::from_bits_retain(self.bits()),
                    TestOverlappingFull::from_bits_retain(self.bits()),
                )
            }
        }
        impl crate::__private::core::iter::IntoIterator for InternalBitFlags {
            type Item = TestOverlappingFull;
            type IntoIter = crate::iter::Iter<TestOverlappingFull>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
        impl InternalBitFlags {
            /// Returns a mutable reference to the raw value of the flags currently stored.
            #[inline]
            pub fn bits_mut(&mut self) -> &mut u8 {
                &mut self.0
            }
        }
        #[allow(dead_code, deprecated, unused_attributes)]
        impl TestOverlappingFull {
            /// Get a flags value with all bits unset.
            #[inline]
            pub const fn empty() -> Self {
                { Self(InternalBitFlags::empty()) }
            }
            /// Get a flags value with all known bits set.
            #[inline]
            pub const fn all() -> Self {
                { Self(InternalBitFlags::all()) }
            }
            /// Get the underlying bits value.
            ///
            /// The returned value is exactly the bits set in this flags value.
            #[inline]
            pub const fn bits(&self) -> u8 {
                let f = self;
                { f.0.bits() }
            }
            /// Convert from a bits value.
            ///
            /// This method will return `None` if any unknown bits are set.
            #[inline]
            pub const fn from_bits(
                bits: u8,
            ) -> crate::__private::core::option::Option<Self> {
                let bits = bits;
                {
                    match InternalBitFlags::from_bits(bits) {
                        crate::__private::core::option::Option::Some(bits) => {
                            crate::__private::core::option::Option::Some(Self(bits))
                        }
                        crate::__private::core::option::Option::None => {
                            crate::__private::core::option::Option::None
                        }
                    }
                }
            }
            /// Convert from a bits value, unsetting any unknown bits.
            #[inline]
            pub const fn from_bits_truncate(bits: u8) -> Self {
                let bits = bits;
                { Self(InternalBitFlags::from_bits_truncate(bits)) }
            }
            /// Convert from a bits value exactly.
            #[inline]
            pub const fn from_bits_retain(bits: u8) -> Self {
                let bits = bits;
                { Self(InternalBitFlags::from_bits_retain(bits)) }
            }
            /// Get a flags value with the bits of a flag with the given name set.
            ///
            /// This method will return `None` if `name` is empty or doesn't
            /// correspond to any named flag.
            #[inline]
            pub fn from_name(
                name: &str,
            ) -> crate::__private::core::option::Option<Self> {
                let name = name;
                {
                    match InternalBitFlags::from_name(name) {
                        crate::__private::core::option::Option::Some(bits) => {
                            crate::__private::core::option::Option::Some(Self(bits))
                        }
                        crate::__private::core::option::Option::None => {
                            crate::__private::core::option::Option::None
                        }
                    }
                }
            }
            /// Whether all bits in this flags value are unset.
            #[inline]
            pub const fn is_empty(&self) -> bool {
                let f = self;
                { f.0.is_empty() }
            }
            /// Whether all known bits in this flags value are set.
            #[inline]
            pub const fn is_all(&self) -> bool {
                let f = self;
                { f.0.is_all() }
            }
            /// Whether any set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn intersects(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.0.intersects(other.0) }
            }
            /// Whether all set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn contains(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.0.contains(other.0) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            pub fn insert(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.insert(other.0) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `remove` won't truncate `other`, but the `!` operator will.
            #[inline]
            pub fn remove(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.remove(other.0) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            pub fn toggle(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.toggle(other.0) }
            }
            /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
            #[inline]
            pub fn set(&mut self, other: Self, value: bool) {
                let f = self;
                let other = other;
                let value = value;
                { f.0.set(other.0, value) }
            }
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.intersection(other.0)) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn union(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.union(other.0)) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            #[must_use]
            pub const fn difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.difference(other.0)) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn symmetric_difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.symmetric_difference(other.0)) }
            }
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            #[must_use]
            pub const fn complement(self) -> Self {
                let f = self;
                { Self(f.0.complement()) }
            }
        }
        impl crate::__private::core::fmt::Binary for TestOverlappingFull {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Binary::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::Octal for TestOverlappingFull {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Octal::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::LowerHex for TestOverlappingFull {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::LowerHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::UpperHex for TestOverlappingFull {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::UpperHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::ops::BitOr for TestOverlappingFull {
            type Output = Self;
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor(self, other: TestOverlappingFull) -> Self {
                self.union(other)
            }
        }
        impl crate::__private::core::ops::BitOrAssign for TestOverlappingFull {
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor_assign(&mut self, other: Self) {
                self.insert(other);
            }
        }
        impl crate::__private::core::ops::BitXor for TestOverlappingFull {
            type Output = Self;
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor(self, other: Self) -> Self {
                self.symmetric_difference(other)
            }
        }
        impl crate::__private::core::ops::BitXorAssign for TestOverlappingFull {
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor_assign(&mut self, other: Self) {
                self.toggle(other);
            }
        }
        impl crate::__private::core::ops::BitAnd for TestOverlappingFull {
            type Output = Self;
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand(self, other: Self) -> Self {
                self.intersection(other)
            }
        }
        impl crate::__private::core::ops::BitAndAssign for TestOverlappingFull {
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand_assign(&mut self, other: Self) {
                *self = Self::from_bits_retain(self.bits()).intersection(other);
            }
        }
        impl crate::__private::core::ops::Sub for TestOverlappingFull {
            type Output = Self;
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub(self, other: Self) -> Self {
                self.difference(other)
            }
        }
        impl crate::__private::core::ops::SubAssign for TestOverlappingFull {
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                self.remove(other);
            }
        }
        impl crate::__private::core::ops::Not for TestOverlappingFull {
            type Output = Self;
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            fn not(self) -> Self {
                self.complement()
            }
        }
        impl crate::__private::core::iter::Extend<TestOverlappingFull>
        for TestOverlappingFull {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn extend<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                &mut self,
                iterator: T,
            ) {
                for item in iterator {
                    self.insert(item)
                }
            }
        }
        impl crate::__private::core::iter::FromIterator<TestOverlappingFull>
        for TestOverlappingFull {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn from_iter<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                iterator: T,
            ) -> Self {
                use crate::__private::core::iter::Extend;
                let mut result = Self::empty();
                result.extend(iterator);
                result
            }
        }
        impl TestOverlappingFull {
            /// Yield a set of contained flags values.
            ///
            /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
            /// will be yielded together as a final flags value.
            #[inline]
            pub const fn iter(&self) -> crate::iter::Iter<TestOverlappingFull> {
                crate::iter::Iter::__private_const_new(
                    <TestOverlappingFull as crate::Flags>::FLAGS,
                    TestOverlappingFull::from_bits_retain(self.bits()),
                    TestOverlappingFull::from_bits_retain(self.bits()),
                )
            }
            /// Yield a set of contained named flags values.
            ///
            /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
            /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
            #[inline]
            pub const fn iter_names(
                &self,
            ) -> crate::iter::IterNames<TestOverlappingFull> {
                crate::iter::IterNames::__private_const_new(
                    <TestOverlappingFull as crate::Flags>::FLAGS,
                    TestOverlappingFull::from_bits_retain(self.bits()),
                    TestOverlappingFull::from_bits_retain(self.bits()),
                )
            }
        }
        impl crate::__private::core::iter::IntoIterator for TestOverlappingFull {
            type Item = TestOverlappingFull;
            type IntoIter = crate::iter::Iter<TestOverlappingFull>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
    };
    pub struct TestExternal(<TestExternal as crate::__private::PublicFlags>::Internal);
    #[automatically_derived]
    impl ::core::fmt::Debug for TestExternal {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_tuple_field1_finish(
                f,
                "TestExternal",
                &&self.0,
            )
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for TestExternal {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for TestExternal {
        #[inline]
        fn eq(&self, other: &TestExternal) -> bool {
            self.0 == other.0
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for TestExternal {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<
                <TestExternal as crate::__private::PublicFlags>::Internal,
            >;
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for TestExternal {
        #[inline]
        fn partial_cmp(
            &self,
            other: &TestExternal,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for TestExternal {
        #[inline]
        fn cmp(&self, other: &TestExternal) -> ::core::cmp::Ordering {
            ::core::cmp::Ord::cmp(&self.0, &other.0)
        }
    }
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for TestExternal {}
    #[automatically_derived]
    impl ::core::clone::Clone for TestExternal {
        #[inline]
        fn clone(&self) -> TestExternal {
            let _: ::core::clone::AssertParamIsClone<
                <TestExternal as crate::__private::PublicFlags>::Internal,
            >;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for TestExternal {}
    impl TestExternal {
        /// 1
        #[allow(deprecated, non_upper_case_globals)]
        pub const A: Self = Self::from_bits_retain(1);
        /// 1 << 1
        #[allow(deprecated, non_upper_case_globals)]
        pub const B: Self = Self::from_bits_retain(1 << 1);
        /// 1 << 2
        #[allow(deprecated, non_upper_case_globals)]
        pub const C: Self = Self::from_bits_retain(1 << 2);
        /// 1 | (1 << 1) | (1 << 2)
        #[allow(deprecated, non_upper_case_globals)]
        pub const ABC: Self = Self::from_bits_retain(
            Self::A.bits() | Self::B.bits() | Self::C.bits(),
        );
    }
    impl crate::Flags for TestExternal {
        const FLAGS: &'static [crate::Flag<TestExternal>] = &[
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("A", TestExternal::A)
            },
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("B", TestExternal::B)
            },
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("C", TestExternal::C)
            },
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("ABC", TestExternal::ABC)
            },
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("", TestExternal::from_bits_retain(!0))
            },
        ];
        type Bits = u8;
        fn bits(&self) -> u8 {
            TestExternal::bits(self)
        }
        fn from_bits_retain(bits: u8) -> TestExternal {
            TestExternal::from_bits_retain(bits)
        }
    }
    #[allow(
        dead_code,
        deprecated,
        unused_doc_comments,
        unused_attributes,
        unused_mut,
        unused_imports,
        non_upper_case_globals,
        clippy::assign_op_pattern,
        clippy::indexing_slicing,
        clippy::same_name_method,
        clippy::iter_without_into_iter,
    )]
    const _: () = {
        #[repr(transparent)]
        pub struct InternalBitFlags(u8);
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::clone::Clone for InternalBitFlags {
            #[inline]
            fn clone(&self) -> InternalBitFlags {
                let _: ::core::clone::AssertParamIsClone<u8>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for InternalBitFlags {
            #[inline]
            fn eq(&self, other: &InternalBitFlags) -> bool {
                self.0 == other.0
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for InternalBitFlags {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<u8>;
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for InternalBitFlags {
            #[inline]
            fn partial_cmp(
                &self,
                other: &InternalBitFlags,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for InternalBitFlags {
            #[inline]
            fn cmp(&self, other: &InternalBitFlags) -> ::core::cmp::Ordering {
                ::core::cmp::Ord::cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for InternalBitFlags {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                ::core::hash::Hash::hash(&self.0, state)
            }
        }
        impl crate::__private::PublicFlags for TestExternal {
            type Primitive = u8;
            type Internal = InternalBitFlags;
        }
        impl crate::__private::core::default::Default for InternalBitFlags {
            #[inline]
            fn default() -> Self {
                InternalBitFlags::empty()
            }
        }
        impl crate::__private::core::fmt::Debug for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter<'_>,
            ) -> crate::__private::core::fmt::Result {
                if self.is_empty() {
                    f.write_fmt(format_args!("{0:#x}", <u8 as crate::Bits>::EMPTY))
                } else {
                    crate::__private::core::fmt::Display::fmt(self, f)
                }
            }
        }
        impl crate::__private::core::fmt::Display for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter<'_>,
            ) -> crate::__private::core::fmt::Result {
                crate::parser::to_writer(&TestExternal(*self), f)
            }
        }
        impl crate::__private::core::str::FromStr for InternalBitFlags {
            type Err = crate::parser::ParseError;
            fn from_str(
                s: &str,
            ) -> crate::__private::core::result::Result<Self, Self::Err> {
                crate::parser::from_str::<TestExternal>(s).map(|flags| flags.0)
            }
        }
        impl crate::__private::core::convert::AsRef<u8> for InternalBitFlags {
            fn as_ref(&self) -> &u8 {
                &self.0
            }
        }
        impl crate::__private::core::convert::From<u8> for InternalBitFlags {
            fn from(bits: u8) -> Self {
                Self::from_bits_retain(bits)
            }
        }
        #[allow(dead_code, deprecated, unused_attributes)]
        impl InternalBitFlags {
            /// Get a flags value with all bits unset.
            #[inline]
            pub const fn empty() -> Self {
                { Self(<u8 as crate::Bits>::EMPTY) }
            }
            /// Get a flags value with all known bits set.
            #[inline]
            pub const fn all() -> Self {
                {
                    let mut truncated = <u8 as crate::Bits>::EMPTY;
                    let mut i = 0;
                    {
                        {
                            let flag = <TestExternal as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    {
                        {
                            let flag = <TestExternal as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    {
                        {
                            let flag = <TestExternal as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    {
                        {
                            let flag = <TestExternal as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    {
                        {
                            let flag = <TestExternal as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    let _ = i;
                    Self::from_bits_retain(truncated)
                }
            }
            /// Get the underlying bits value.
            ///
            /// The returned value is exactly the bits set in this flags value.
            #[inline]
            pub const fn bits(&self) -> u8 {
                let f = self;
                { f.0 }
            }
            /// Convert from a bits value.
            ///
            /// This method will return `None` if any unknown bits are set.
            #[inline]
            pub const fn from_bits(
                bits: u8,
            ) -> crate::__private::core::option::Option<Self> {
                let bits = bits;
                {
                    let truncated = Self::from_bits_truncate(bits).0;
                    if truncated == bits {
                        crate::__private::core::option::Option::Some(Self(bits))
                    } else {
                        crate::__private::core::option::Option::None
                    }
                }
            }
            /// Convert from a bits value, unsetting any unknown bits.
            #[inline]
            pub const fn from_bits_truncate(bits: u8) -> Self {
                let bits = bits;
                { Self(bits & Self::all().bits()) }
            }
            /// Convert from a bits value exactly.
            #[inline]
            pub const fn from_bits_retain(bits: u8) -> Self {
                let bits = bits;
                { Self(bits) }
            }
            /// Get a flags value with the bits of a flag with the given name set.
            ///
            /// This method will return `None` if `name` is empty or doesn't
            /// correspond to any named flag.
            #[inline]
            pub fn from_name(
                name: &str,
            ) -> crate::__private::core::option::Option<Self> {
                let name = name;
                {
                    {
                        if name == "A" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestExternal::A.bits()),
                            );
                        }
                    };
                    {
                        if name == "B" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestExternal::B.bits()),
                            );
                        }
                    };
                    {
                        if name == "C" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestExternal::C.bits()),
                            );
                        }
                    };
                    {
                        if name == "ABC" {
                            return crate::__private::core::option::Option::Some(
                                Self(TestExternal::ABC.bits()),
                            );
                        }
                    };
                    let _ = name;
                    crate::__private::core::option::Option::None
                }
            }
            /// Whether all bits in this flags value are unset.
            #[inline]
            pub const fn is_empty(&self) -> bool {
                let f = self;
                { f.bits() == <u8 as crate::Bits>::EMPTY }
            }
            /// Whether all known bits in this flags value are set.
            #[inline]
            pub const fn is_all(&self) -> bool {
                let f = self;
                { Self::all().bits() | f.bits() == f.bits() }
            }
            /// Whether any set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn intersects(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.bits() & other.bits() != <u8 as crate::Bits>::EMPTY }
            }
            /// Whether all set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn contains(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.bits() & other.bits() == other.bits() }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            pub fn insert(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).union(other);
                }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `remove` won't truncate `other`, but the `!` operator will.
            #[inline]
            pub fn remove(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).difference(other);
                }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            pub fn toggle(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).symmetric_difference(other);
                }
            }
            /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
            #[inline]
            pub fn set(&mut self, other: Self, value: bool) {
                let f = self;
                let other = other;
                let value = value;
                {
                    if value {
                        f.insert(other);
                    } else {
                        f.remove(other);
                    }
                }
            }
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() & other.bits()) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn union(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() | other.bits()) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            #[must_use]
            pub const fn difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() & !other.bits()) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn symmetric_difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() ^ other.bits()) }
            }
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            #[must_use]
            pub const fn complement(self) -> Self {
                let f = self;
                { Self::from_bits_truncate(!f.bits()) }
            }
        }
        impl crate::__private::core::fmt::Binary for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Binary::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::Octal for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Octal::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::LowerHex for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::LowerHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::UpperHex for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::UpperHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::ops::BitOr for InternalBitFlags {
            type Output = Self;
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor(self, other: InternalBitFlags) -> Self {
                self.union(other)
            }
        }
        impl crate::__private::core::ops::BitOrAssign for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor_assign(&mut self, other: Self) {
                self.insert(other);
            }
        }
        impl crate::__private::core::ops::BitXor for InternalBitFlags {
            type Output = Self;
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor(self, other: Self) -> Self {
                self.symmetric_difference(other)
            }
        }
        impl crate::__private::core::ops::BitXorAssign for InternalBitFlags {
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor_assign(&mut self, other: Self) {
                self.toggle(other);
            }
        }
        impl crate::__private::core::ops::BitAnd for InternalBitFlags {
            type Output = Self;
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand(self, other: Self) -> Self {
                self.intersection(other)
            }
        }
        impl crate::__private::core::ops::BitAndAssign for InternalBitFlags {
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand_assign(&mut self, other: Self) {
                *self = Self::from_bits_retain(self.bits()).intersection(other);
            }
        }
        impl crate::__private::core::ops::Sub for InternalBitFlags {
            type Output = Self;
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub(self, other: Self) -> Self {
                self.difference(other)
            }
        }
        impl crate::__private::core::ops::SubAssign for InternalBitFlags {
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                self.remove(other);
            }
        }
        impl crate::__private::core::ops::Not for InternalBitFlags {
            type Output = Self;
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            fn not(self) -> Self {
                self.complement()
            }
        }
        impl crate::__private::core::iter::Extend<InternalBitFlags>
        for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn extend<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                &mut self,
                iterator: T,
            ) {
                for item in iterator {
                    self.insert(item)
                }
            }
        }
        impl crate::__private::core::iter::FromIterator<InternalBitFlags>
        for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn from_iter<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                iterator: T,
            ) -> Self {
                use crate::__private::core::iter::Extend;
                let mut result = Self::empty();
                result.extend(iterator);
                result
            }
        }
        impl InternalBitFlags {
            /// Yield a set of contained flags values.
            ///
            /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
            /// will be yielded together as a final flags value.
            #[inline]
            pub const fn iter(&self) -> crate::iter::Iter<TestExternal> {
                crate::iter::Iter::__private_const_new(
                    <TestExternal as crate::Flags>::FLAGS,
                    TestExternal::from_bits_retain(self.bits()),
                    TestExternal::from_bits_retain(self.bits()),
                )
            }
            /// Yield a set of contained named flags values.
            ///
            /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
            /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
            #[inline]
            pub const fn iter_names(&self) -> crate::iter::IterNames<TestExternal> {
                crate::iter::IterNames::__private_const_new(
                    <TestExternal as crate::Flags>::FLAGS,
                    TestExternal::from_bits_retain(self.bits()),
                    TestExternal::from_bits_retain(self.bits()),
                )
            }
        }
        impl crate::__private::core::iter::IntoIterator for InternalBitFlags {
            type Item = TestExternal;
            type IntoIter = crate::iter::Iter<TestExternal>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
        impl InternalBitFlags {
            /// Returns a mutable reference to the raw value of the flags currently stored.
            #[inline]
            pub fn bits_mut(&mut self) -> &mut u8 {
                &mut self.0
            }
        }
        #[allow(dead_code, deprecated, unused_attributes)]
        impl TestExternal {
            /// Get a flags value with all bits unset.
            #[inline]
            pub const fn empty() -> Self {
                { Self(InternalBitFlags::empty()) }
            }
            /// Get a flags value with all known bits set.
            #[inline]
            pub const fn all() -> Self {
                { Self(InternalBitFlags::all()) }
            }
            /// Get the underlying bits value.
            ///
            /// The returned value is exactly the bits set in this flags value.
            #[inline]
            pub const fn bits(&self) -> u8 {
                let f = self;
                { f.0.bits() }
            }
            /// Convert from a bits value.
            ///
            /// This method will return `None` if any unknown bits are set.
            #[inline]
            pub const fn from_bits(
                bits: u8,
            ) -> crate::__private::core::option::Option<Self> {
                let bits = bits;
                {
                    match InternalBitFlags::from_bits(bits) {
                        crate::__private::core::option::Option::Some(bits) => {
                            crate::__private::core::option::Option::Some(Self(bits))
                        }
                        crate::__private::core::option::Option::None => {
                            crate::__private::core::option::Option::None
                        }
                    }
                }
            }
            /// Convert from a bits value, unsetting any unknown bits.
            #[inline]
            pub const fn from_bits_truncate(bits: u8) -> Self {
                let bits = bits;
                { Self(InternalBitFlags::from_bits_truncate(bits)) }
            }
            /// Convert from a bits value exactly.
            #[inline]
            pub const fn from_bits_retain(bits: u8) -> Self {
                let bits = bits;
                { Self(InternalBitFlags::from_bits_retain(bits)) }
            }
            /// Get a flags value with the bits of a flag with the given name set.
            ///
            /// This method will return `None` if `name` is empty or doesn't
            /// correspond to any named flag.
            #[inline]
            pub fn from_name(
                name: &str,
            ) -> crate::__private::core::option::Option<Self> {
                let name = name;
                {
                    match InternalBitFlags::from_name(name) {
                        crate::__private::core::option::Option::Some(bits) => {
                            crate::__private::core::option::Option::Some(Self(bits))
                        }
                        crate::__private::core::option::Option::None => {
                            crate::__private::core::option::Option::None
                        }
                    }
                }
            }
            /// Whether all bits in this flags value are unset.
            #[inline]
            pub const fn is_empty(&self) -> bool {
                let f = self;
                { f.0.is_empty() }
            }
            /// Whether all known bits in this flags value are set.
            #[inline]
            pub const fn is_all(&self) -> bool {
                let f = self;
                { f.0.is_all() }
            }
            /// Whether any set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn intersects(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.0.intersects(other.0) }
            }
            /// Whether all set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn contains(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.0.contains(other.0) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            pub fn insert(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.insert(other.0) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `remove` won't truncate `other`, but the `!` operator will.
            #[inline]
            pub fn remove(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.remove(other.0) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            pub fn toggle(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.toggle(other.0) }
            }
            /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
            #[inline]
            pub fn set(&mut self, other: Self, value: bool) {
                let f = self;
                let other = other;
                let value = value;
                { f.0.set(other.0, value) }
            }
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.intersection(other.0)) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn union(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.union(other.0)) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            #[must_use]
            pub const fn difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.difference(other.0)) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn symmetric_difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.symmetric_difference(other.0)) }
            }
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            #[must_use]
            pub const fn complement(self) -> Self {
                let f = self;
                { Self(f.0.complement()) }
            }
        }
        impl crate::__private::core::fmt::Binary for TestExternal {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Binary::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::Octal for TestExternal {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Octal::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::LowerHex for TestExternal {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::LowerHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::UpperHex for TestExternal {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::UpperHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::ops::BitOr for TestExternal {
            type Output = Self;
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor(self, other: TestExternal) -> Self {
                self.union(other)
            }
        }
        impl crate::__private::core::ops::BitOrAssign for TestExternal {
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor_assign(&mut self, other: Self) {
                self.insert(other);
            }
        }
        impl crate::__private::core::ops::BitXor for TestExternal {
            type Output = Self;
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor(self, other: Self) -> Self {
                self.symmetric_difference(other)
            }
        }
        impl crate::__private::core::ops::BitXorAssign for TestExternal {
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor_assign(&mut self, other: Self) {
                self.toggle(other);
            }
        }
        impl crate::__private::core::ops::BitAnd for TestExternal {
            type Output = Self;
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand(self, other: Self) -> Self {
                self.intersection(other)
            }
        }
        impl crate::__private::core::ops::BitAndAssign for TestExternal {
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand_assign(&mut self, other: Self) {
                *self = Self::from_bits_retain(self.bits()).intersection(other);
            }
        }
        impl crate::__private::core::ops::Sub for TestExternal {
            type Output = Self;
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub(self, other: Self) -> Self {
                self.difference(other)
            }
        }
        impl crate::__private::core::ops::SubAssign for TestExternal {
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                self.remove(other);
            }
        }
        impl crate::__private::core::ops::Not for TestExternal {
            type Output = Self;
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            fn not(self) -> Self {
                self.complement()
            }
        }
        impl crate::__private::core::iter::Extend<TestExternal> for TestExternal {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn extend<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                &mut self,
                iterator: T,
            ) {
                for item in iterator {
                    self.insert(item)
                }
            }
        }
        impl crate::__private::core::iter::FromIterator<TestExternal> for TestExternal {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn from_iter<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                iterator: T,
            ) -> Self {
                use crate::__private::core::iter::Extend;
                let mut result = Self::empty();
                result.extend(iterator);
                result
            }
        }
        impl TestExternal {
            /// Yield a set of contained flags values.
            ///
            /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
            /// will be yielded together as a final flags value.
            #[inline]
            pub const fn iter(&self) -> crate::iter::Iter<TestExternal> {
                crate::iter::Iter::__private_const_new(
                    <TestExternal as crate::Flags>::FLAGS,
                    TestExternal::from_bits_retain(self.bits()),
                    TestExternal::from_bits_retain(self.bits()),
                )
            }
            /// Yield a set of contained named flags values.
            ///
            /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
            /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
            #[inline]
            pub const fn iter_names(&self) -> crate::iter::IterNames<TestExternal> {
                crate::iter::IterNames::__private_const_new(
                    <TestExternal as crate::Flags>::FLAGS,
                    TestExternal::from_bits_retain(self.bits()),
                    TestExternal::from_bits_retain(self.bits()),
                )
            }
        }
        impl crate::__private::core::iter::IntoIterator for TestExternal {
            type Item = TestExternal;
            type IntoIter = crate::iter::Iter<TestExternal>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
    };
    pub struct TestExternalFull(
        <TestExternalFull as crate::__private::PublicFlags>::Internal,
    );
    #[automatically_derived]
    impl ::core::fmt::Debug for TestExternalFull {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_tuple_field1_finish(
                f,
                "TestExternalFull",
                &&self.0,
            )
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for TestExternalFull {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for TestExternalFull {
        #[inline]
        fn eq(&self, other: &TestExternalFull) -> bool {
            self.0 == other.0
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for TestExternalFull {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<
                <TestExternalFull as crate::__private::PublicFlags>::Internal,
            >;
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for TestExternalFull {
        #[inline]
        fn partial_cmp(
            &self,
            other: &TestExternalFull,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for TestExternalFull {
        #[inline]
        fn cmp(&self, other: &TestExternalFull) -> ::core::cmp::Ordering {
            ::core::cmp::Ord::cmp(&self.0, &other.0)
        }
    }
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for TestExternalFull {}
    #[automatically_derived]
    impl ::core::clone::Clone for TestExternalFull {
        #[inline]
        fn clone(&self) -> TestExternalFull {
            let _: ::core::clone::AssertParamIsClone<
                <TestExternalFull as crate::__private::PublicFlags>::Internal,
            >;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for TestExternalFull {}
    impl TestExternalFull {}
    impl crate::Flags for TestExternalFull {
        const FLAGS: &'static [crate::Flag<TestExternalFull>] = &[
            {
                #[allow(deprecated, non_upper_case_globals)]
                crate::Flag::new("", TestExternalFull::from_bits_retain(!0))
            },
        ];
        type Bits = u8;
        fn bits(&self) -> u8 {
            TestExternalFull::bits(self)
        }
        fn from_bits_retain(bits: u8) -> TestExternalFull {
            TestExternalFull::from_bits_retain(bits)
        }
    }
    #[allow(
        dead_code,
        deprecated,
        unused_doc_comments,
        unused_attributes,
        unused_mut,
        unused_imports,
        non_upper_case_globals,
        clippy::assign_op_pattern,
        clippy::indexing_slicing,
        clippy::same_name_method,
        clippy::iter_without_into_iter,
    )]
    const _: () = {
        #[repr(transparent)]
        pub struct InternalBitFlags(u8);
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::clone::Clone for InternalBitFlags {
            #[inline]
            fn clone(&self) -> InternalBitFlags {
                let _: ::core::clone::AssertParamIsClone<u8>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for InternalBitFlags {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for InternalBitFlags {
            #[inline]
            fn eq(&self, other: &InternalBitFlags) -> bool {
                self.0 == other.0
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for InternalBitFlags {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<u8>;
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for InternalBitFlags {
            #[inline]
            fn partial_cmp(
                &self,
                other: &InternalBitFlags,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for InternalBitFlags {
            #[inline]
            fn cmp(&self, other: &InternalBitFlags) -> ::core::cmp::Ordering {
                ::core::cmp::Ord::cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for InternalBitFlags {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                ::core::hash::Hash::hash(&self.0, state)
            }
        }
        impl crate::__private::PublicFlags for TestExternalFull {
            type Primitive = u8;
            type Internal = InternalBitFlags;
        }
        impl crate::__private::core::default::Default for InternalBitFlags {
            #[inline]
            fn default() -> Self {
                InternalBitFlags::empty()
            }
        }
        impl crate::__private::core::fmt::Debug for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter<'_>,
            ) -> crate::__private::core::fmt::Result {
                if self.is_empty() {
                    f.write_fmt(format_args!("{0:#x}", <u8 as crate::Bits>::EMPTY))
                } else {
                    crate::__private::core::fmt::Display::fmt(self, f)
                }
            }
        }
        impl crate::__private::core::fmt::Display for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter<'_>,
            ) -> crate::__private::core::fmt::Result {
                crate::parser::to_writer(&TestExternalFull(*self), f)
            }
        }
        impl crate::__private::core::str::FromStr for InternalBitFlags {
            type Err = crate::parser::ParseError;
            fn from_str(
                s: &str,
            ) -> crate::__private::core::result::Result<Self, Self::Err> {
                crate::parser::from_str::<TestExternalFull>(s).map(|flags| flags.0)
            }
        }
        impl crate::__private::core::convert::AsRef<u8> for InternalBitFlags {
            fn as_ref(&self) -> &u8 {
                &self.0
            }
        }
        impl crate::__private::core::convert::From<u8> for InternalBitFlags {
            fn from(bits: u8) -> Self {
                Self::from_bits_retain(bits)
            }
        }
        #[allow(dead_code, deprecated, unused_attributes)]
        impl InternalBitFlags {
            /// Get a flags value with all bits unset.
            #[inline]
            pub const fn empty() -> Self {
                { Self(<u8 as crate::Bits>::EMPTY) }
            }
            /// Get a flags value with all known bits set.
            #[inline]
            pub const fn all() -> Self {
                {
                    let mut truncated = <u8 as crate::Bits>::EMPTY;
                    let mut i = 0;
                    {
                        {
                            let flag = <TestExternalFull as crate::Flags>::FLAGS[i]
                                .value()
                                .bits();
                            truncated = truncated | flag;
                            i += 1;
                        }
                    };
                    let _ = i;
                    Self::from_bits_retain(truncated)
                }
            }
            /// Get the underlying bits value.
            ///
            /// The returned value is exactly the bits set in this flags value.
            #[inline]
            pub const fn bits(&self) -> u8 {
                let f = self;
                { f.0 }
            }
            /// Convert from a bits value.
            ///
            /// This method will return `None` if any unknown bits are set.
            #[inline]
            pub const fn from_bits(
                bits: u8,
            ) -> crate::__private::core::option::Option<Self> {
                let bits = bits;
                {
                    let truncated = Self::from_bits_truncate(bits).0;
                    if truncated == bits {
                        crate::__private::core::option::Option::Some(Self(bits))
                    } else {
                        crate::__private::core::option::Option::None
                    }
                }
            }
            /// Convert from a bits value, unsetting any unknown bits.
            #[inline]
            pub const fn from_bits_truncate(bits: u8) -> Self {
                let bits = bits;
                { Self(bits & Self::all().bits()) }
            }
            /// Convert from a bits value exactly.
            #[inline]
            pub const fn from_bits_retain(bits: u8) -> Self {
                let bits = bits;
                { Self(bits) }
            }
            /// Get a flags value with the bits of a flag with the given name set.
            ///
            /// This method will return `None` if `name` is empty or doesn't
            /// correspond to any named flag.
            #[inline]
            pub fn from_name(
                name: &str,
            ) -> crate::__private::core::option::Option<Self> {
                let name = name;
                {
                    let _ = name;
                    crate::__private::core::option::Option::None
                }
            }
            /// Whether all bits in this flags value are unset.
            #[inline]
            pub const fn is_empty(&self) -> bool {
                let f = self;
                { f.bits() == <u8 as crate::Bits>::EMPTY }
            }
            /// Whether all known bits in this flags value are set.
            #[inline]
            pub const fn is_all(&self) -> bool {
                let f = self;
                { Self::all().bits() | f.bits() == f.bits() }
            }
            /// Whether any set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn intersects(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.bits() & other.bits() != <u8 as crate::Bits>::EMPTY }
            }
            /// Whether all set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn contains(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.bits() & other.bits() == other.bits() }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            pub fn insert(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).union(other);
                }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `remove` won't truncate `other`, but the `!` operator will.
            #[inline]
            pub fn remove(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).difference(other);
                }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            pub fn toggle(&mut self, other: Self) {
                let f = self;
                let other = other;
                {
                    *f = Self::from_bits_retain(f.bits()).symmetric_difference(other);
                }
            }
            /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
            #[inline]
            pub fn set(&mut self, other: Self, value: bool) {
                let f = self;
                let other = other;
                let value = value;
                {
                    if value {
                        f.insert(other);
                    } else {
                        f.remove(other);
                    }
                }
            }
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() & other.bits()) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn union(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() | other.bits()) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            #[must_use]
            pub const fn difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() & !other.bits()) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn symmetric_difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self::from_bits_retain(f.bits() ^ other.bits()) }
            }
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            #[must_use]
            pub const fn complement(self) -> Self {
                let f = self;
                { Self::from_bits_truncate(!f.bits()) }
            }
        }
        impl crate::__private::core::fmt::Binary for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Binary::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::Octal for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Octal::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::LowerHex for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::LowerHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::UpperHex for InternalBitFlags {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::UpperHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::ops::BitOr for InternalBitFlags {
            type Output = Self;
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor(self, other: InternalBitFlags) -> Self {
                self.union(other)
            }
        }
        impl crate::__private::core::ops::BitOrAssign for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor_assign(&mut self, other: Self) {
                self.insert(other);
            }
        }
        impl crate::__private::core::ops::BitXor for InternalBitFlags {
            type Output = Self;
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor(self, other: Self) -> Self {
                self.symmetric_difference(other)
            }
        }
        impl crate::__private::core::ops::BitXorAssign for InternalBitFlags {
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor_assign(&mut self, other: Self) {
                self.toggle(other);
            }
        }
        impl crate::__private::core::ops::BitAnd for InternalBitFlags {
            type Output = Self;
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand(self, other: Self) -> Self {
                self.intersection(other)
            }
        }
        impl crate::__private::core::ops::BitAndAssign for InternalBitFlags {
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand_assign(&mut self, other: Self) {
                *self = Self::from_bits_retain(self.bits()).intersection(other);
            }
        }
        impl crate::__private::core::ops::Sub for InternalBitFlags {
            type Output = Self;
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub(self, other: Self) -> Self {
                self.difference(other)
            }
        }
        impl crate::__private::core::ops::SubAssign for InternalBitFlags {
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                self.remove(other);
            }
        }
        impl crate::__private::core::ops::Not for InternalBitFlags {
            type Output = Self;
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            fn not(self) -> Self {
                self.complement()
            }
        }
        impl crate::__private::core::iter::Extend<InternalBitFlags>
        for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn extend<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                &mut self,
                iterator: T,
            ) {
                for item in iterator {
                    self.insert(item)
                }
            }
        }
        impl crate::__private::core::iter::FromIterator<InternalBitFlags>
        for InternalBitFlags {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn from_iter<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                iterator: T,
            ) -> Self {
                use crate::__private::core::iter::Extend;
                let mut result = Self::empty();
                result.extend(iterator);
                result
            }
        }
        impl InternalBitFlags {
            /// Yield a set of contained flags values.
            ///
            /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
            /// will be yielded together as a final flags value.
            #[inline]
            pub const fn iter(&self) -> crate::iter::Iter<TestExternalFull> {
                crate::iter::Iter::__private_const_new(
                    <TestExternalFull as crate::Flags>::FLAGS,
                    TestExternalFull::from_bits_retain(self.bits()),
                    TestExternalFull::from_bits_retain(self.bits()),
                )
            }
            /// Yield a set of contained named flags values.
            ///
            /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
            /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
            #[inline]
            pub const fn iter_names(&self) -> crate::iter::IterNames<TestExternalFull> {
                crate::iter::IterNames::__private_const_new(
                    <TestExternalFull as crate::Flags>::FLAGS,
                    TestExternalFull::from_bits_retain(self.bits()),
                    TestExternalFull::from_bits_retain(self.bits()),
                )
            }
        }
        impl crate::__private::core::iter::IntoIterator for InternalBitFlags {
            type Item = TestExternalFull;
            type IntoIter = crate::iter::Iter<TestExternalFull>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
        impl InternalBitFlags {
            /// Returns a mutable reference to the raw value of the flags currently stored.
            #[inline]
            pub fn bits_mut(&mut self) -> &mut u8 {
                &mut self.0
            }
        }
        #[allow(dead_code, deprecated, unused_attributes)]
        impl TestExternalFull {
            /// Get a flags value with all bits unset.
            #[inline]
            pub const fn empty() -> Self {
                { Self(InternalBitFlags::empty()) }
            }
            /// Get a flags value with all known bits set.
            #[inline]
            pub const fn all() -> Self {
                { Self(InternalBitFlags::all()) }
            }
            /// Get the underlying bits value.
            ///
            /// The returned value is exactly the bits set in this flags value.
            #[inline]
            pub const fn bits(&self) -> u8 {
                let f = self;
                { f.0.bits() }
            }
            /// Convert from a bits value.
            ///
            /// This method will return `None` if any unknown bits are set.
            #[inline]
            pub const fn from_bits(
                bits: u8,
            ) -> crate::__private::core::option::Option<Self> {
                let bits = bits;
                {
                    match InternalBitFlags::from_bits(bits) {
                        crate::__private::core::option::Option::Some(bits) => {
                            crate::__private::core::option::Option::Some(Self(bits))
                        }
                        crate::__private::core::option::Option::None => {
                            crate::__private::core::option::Option::None
                        }
                    }
                }
            }
            /// Convert from a bits value, unsetting any unknown bits.
            #[inline]
            pub const fn from_bits_truncate(bits: u8) -> Self {
                let bits = bits;
                { Self(InternalBitFlags::from_bits_truncate(bits)) }
            }
            /// Convert from a bits value exactly.
            #[inline]
            pub const fn from_bits_retain(bits: u8) -> Self {
                let bits = bits;
                { Self(InternalBitFlags::from_bits_retain(bits)) }
            }
            /// Get a flags value with the bits of a flag with the given name set.
            ///
            /// This method will return `None` if `name` is empty or doesn't
            /// correspond to any named flag.
            #[inline]
            pub fn from_name(
                name: &str,
            ) -> crate::__private::core::option::Option<Self> {
                let name = name;
                {
                    match InternalBitFlags::from_name(name) {
                        crate::__private::core::option::Option::Some(bits) => {
                            crate::__private::core::option::Option::Some(Self(bits))
                        }
                        crate::__private::core::option::Option::None => {
                            crate::__private::core::option::Option::None
                        }
                    }
                }
            }
            /// Whether all bits in this flags value are unset.
            #[inline]
            pub const fn is_empty(&self) -> bool {
                let f = self;
                { f.0.is_empty() }
            }
            /// Whether all known bits in this flags value are set.
            #[inline]
            pub const fn is_all(&self) -> bool {
                let f = self;
                { f.0.is_all() }
            }
            /// Whether any set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn intersects(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.0.intersects(other.0) }
            }
            /// Whether all set bits in a source flags value are also set in a target flags value.
            #[inline]
            pub const fn contains(&self, other: Self) -> bool {
                let f = self;
                let other = other;
                { f.0.contains(other.0) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            pub fn insert(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.insert(other.0) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `remove` won't truncate `other`, but the `!` operator will.
            #[inline]
            pub fn remove(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.remove(other.0) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            pub fn toggle(&mut self, other: Self) {
                let f = self;
                let other = other;
                { f.0.toggle(other.0) }
            }
            /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
            #[inline]
            pub fn set(&mut self, other: Self, value: bool) {
                let f = self;
                let other = other;
                let value = value;
                { f.0.set(other.0, value) }
            }
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.intersection(other.0)) }
            }
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn union(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.union(other.0)) }
            }
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            #[must_use]
            pub const fn difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.difference(other.0)) }
            }
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            #[must_use]
            pub const fn symmetric_difference(self, other: Self) -> Self {
                let f = self;
                let other = other;
                { Self(f.0.symmetric_difference(other.0)) }
            }
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            #[must_use]
            pub const fn complement(self) -> Self {
                let f = self;
                { Self(f.0.complement()) }
            }
        }
        impl crate::__private::core::fmt::Binary for TestExternalFull {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Binary::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::Octal for TestExternalFull {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::Octal::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::LowerHex for TestExternalFull {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::LowerHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::fmt::UpperHex for TestExternalFull {
            fn fmt(
                &self,
                f: &mut crate::__private::core::fmt::Formatter,
            ) -> crate::__private::core::fmt::Result {
                let inner = self.0;
                crate::__private::core::fmt::UpperHex::fmt(&inner, f)
            }
        }
        impl crate::__private::core::ops::BitOr for TestExternalFull {
            type Output = Self;
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor(self, other: TestExternalFull) -> Self {
                self.union(other)
            }
        }
        impl crate::__private::core::ops::BitOrAssign for TestExternalFull {
            /// The bitwise or (`|`) of the bits in two flags values.
            #[inline]
            fn bitor_assign(&mut self, other: Self) {
                self.insert(other);
            }
        }
        impl crate::__private::core::ops::BitXor for TestExternalFull {
            type Output = Self;
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor(self, other: Self) -> Self {
                self.symmetric_difference(other)
            }
        }
        impl crate::__private::core::ops::BitXorAssign for TestExternalFull {
            /// The bitwise exclusive-or (`^`) of the bits in two flags values.
            #[inline]
            fn bitxor_assign(&mut self, other: Self) {
                self.toggle(other);
            }
        }
        impl crate::__private::core::ops::BitAnd for TestExternalFull {
            type Output = Self;
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand(self, other: Self) -> Self {
                self.intersection(other)
            }
        }
        impl crate::__private::core::ops::BitAndAssign for TestExternalFull {
            /// The bitwise and (`&`) of the bits in two flags values.
            #[inline]
            fn bitand_assign(&mut self, other: Self) {
                *self = Self::from_bits_retain(self.bits()).intersection(other);
            }
        }
        impl crate::__private::core::ops::Sub for TestExternalFull {
            type Output = Self;
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub(self, other: Self) -> Self {
                self.difference(other)
            }
        }
        impl crate::__private::core::ops::SubAssign for TestExternalFull {
            /// The intersection of a source flags value with the complement of a target flags value (`&!`).
            ///
            /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
            /// `difference` won't truncate `other`, but the `!` operator will.
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                self.remove(other);
            }
        }
        impl crate::__private::core::ops::Not for TestExternalFull {
            type Output = Self;
            /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
            #[inline]
            fn not(self) -> Self {
                self.complement()
            }
        }
        impl crate::__private::core::iter::Extend<TestExternalFull>
        for TestExternalFull {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn extend<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                &mut self,
                iterator: T,
            ) {
                for item in iterator {
                    self.insert(item)
                }
            }
        }
        impl crate::__private::core::iter::FromIterator<TestExternalFull>
        for TestExternalFull {
            /// The bitwise or (`|`) of the bits in each flags value.
            fn from_iter<T: crate::__private::core::iter::IntoIterator<Item = Self>>(
                iterator: T,
            ) -> Self {
                use crate::__private::core::iter::Extend;
                let mut result = Self::empty();
                result.extend(iterator);
                result
            }
        }
        impl TestExternalFull {
            /// Yield a set of contained flags values.
            ///
            /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
            /// will be yielded together as a final flags value.
            #[inline]
            pub const fn iter(&self) -> crate::iter::Iter<TestExternalFull> {
                crate::iter::Iter::__private_const_new(
                    <TestExternalFull as crate::Flags>::FLAGS,
                    TestExternalFull::from_bits_retain(self.bits()),
                    TestExternalFull::from_bits_retain(self.bits()),
                )
            }
            /// Yield a set of contained named flags values.
            ///
            /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
            /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
            #[inline]
            pub const fn iter_names(&self) -> crate::iter::IterNames<TestExternalFull> {
                crate::iter::IterNames::__private_const_new(
                    <TestExternalFull as crate::Flags>::FLAGS,
                    TestExternalFull::from_bits_retain(self.bits()),
                    TestExternalFull::from_bits_retain(self.bits()),
                )
            }
        }
        impl crate::__private::core::iter::IntoIterator for TestExternalFull {
            type Item = TestExternalFull;
            type IntoIter = crate::iter::Iter<TestExternalFull>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
    };
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(
        &[
            &cases,
            &cases,
            &cases,
            &cases,
            &cases,
            &cases,
            &cases,
            &cases,
            &cases,
            &cases,
            &cases,
            &cases,
            &cases,
            &cases,
            &cases,
            &cases,
            &cases,
            &cases,
            &cases,
            &cases,
            &cases,
            &cases,
            &cases,
            &cases,
            &roundtrip,
            &invalid,
            &valid,
            &invalid,
            &valid,
            &valid,
            &roundtrip,
            &roundtrip_strict,
            &roundtrip_truncate,
            &cases,
            &cases,
            &cases,
            &cases,
            &cases,
            &cases,
        ],
    )
}
