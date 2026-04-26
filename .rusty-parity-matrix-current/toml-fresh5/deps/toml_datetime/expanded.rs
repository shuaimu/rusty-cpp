#![feature(prelude_import)]
//! A [TOML]-compatible datetime type
//!
//! [TOML]: https://github.com/toml-lang/toml
#![warn(missing_docs)]
#![warn(clippy::std_instead_of_core)]
#![warn(clippy::std_instead_of_alloc)]
#![forbid(unsafe_code)]
#![warn(clippy::print_stderr)]
#![warn(clippy::print_stdout)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
#[allow(unused_extern_crates)]
extern crate alloc;
mod datetime {
    use core::fmt;
    use core::str::{self, FromStr};
    /// A parsed TOML datetime value
    ///
    /// This structure is intended to represent the datetime primitive type that can
    /// be encoded into TOML documents. This type is a parsed version that contains
    /// all metadata internally.
    ///
    /// Currently this type is intentionally conservative and only supports
    /// `to_string` as an accessor. Over time though it's intended that it'll grow
    /// more support!
    ///
    /// Note that if you're using `Deserialize` to deserialize a TOML document, you
    /// can use this as a placeholder for where you're expecting a datetime to be
    /// specified.
    ///
    /// Also note though that while this type implements `Serialize` and
    /// `Deserialize` it's only recommended to use this type with the TOML format,
    /// otherwise encoded in other formats it may look a little odd.
    ///
    /// Depending on how the option values are used, this struct will correspond
    /// with one of the following four datetimes from the [TOML v1.0.0 spec]:
    ///
    /// | `date`    | `time`    | `offset`  | TOML type          |
    /// | --------- | --------- | --------- | ------------------ |
    /// | `Some(_)` | `Some(_)` | `Some(_)` | [Offset Date-Time] |
    /// | `Some(_)` | `Some(_)` | `None`    | [Local Date-Time]  |
    /// | `Some(_)` | `None`    | `None`    | [Local Date]       |
    /// | `None`    | `Some(_)` | `None`    | [Local Time]       |
    ///
    /// **1. Offset Date-Time**: If all the optional values are used, `Datetime`
    /// corresponds to an [Offset Date-Time]. From the TOML v1.0.0 spec:
    ///
    /// > To unambiguously represent a specific instant in time, you may use an
    /// > RFC 3339 formatted date-time with offset.
    /// >
    /// > ```toml
    /// > odt1 = 1979-05-27T07:32:00Z
    /// > odt2 = 1979-05-27T00:32:00-07:00
    /// > odt3 = 1979-05-27T00:32:00.999999-07:00
    /// > ```
    /// >
    /// > For the sake of readability, you may replace the T delimiter between date
    /// > and time with a space character (as permitted by RFC 3339 section 5.6).
    /// >
    /// > ```toml
    /// > odt4 = 1979-05-27 07:32:00Z
    /// > ```
    ///
    /// **2. Local Date-Time**: If `date` and `time` are given but `offset` is
    /// `None`, `Datetime` corresponds to a [Local Date-Time]. From the spec:
    ///
    /// > If you omit the offset from an RFC 3339 formatted date-time, it will
    /// > represent the given date-time without any relation to an offset or
    /// > timezone. It cannot be converted to an instant in time without additional
    /// > information. Conversion to an instant, if required, is implementation-
    /// > specific.
    /// >
    /// > ```toml
    /// > ldt1 = 1979-05-27T07:32:00
    /// > ldt2 = 1979-05-27T00:32:00.999999
    /// > ```
    ///
    /// **3. Local Date**: If only `date` is given, `Datetime` corresponds to a
    /// [Local Date]; see the docs for [`Date`].
    ///
    /// **4. Local Time**: If only `time` is given, `Datetime` corresponds to a
    /// [Local Time]; see the docs for [`Time`].
    ///
    /// [TOML v1.0.0 spec]: https://toml.io/en/v1.0.0
    /// [Offset Date-Time]: https://toml.io/en/v1.0.0#offset-date-time
    /// [Local Date-Time]: https://toml.io/en/v1.0.0#local-date-time
    /// [Local Date]: https://toml.io/en/v1.0.0#local-date
    /// [Local Time]: https://toml.io/en/v1.0.0#local-time
    pub struct Datetime {
        /// Optional date.
        /// Required for: *Offset Date-Time*, *Local Date-Time*, *Local Date*.
        pub date: Option<Date>,
        /// Optional time.
        /// Required for: *Offset Date-Time*, *Local Date-Time*, *Local Time*.
        pub time: Option<Time>,
        /// Optional offset.
        /// Required for: *Offset Date-Time*.
        pub offset: Option<Offset>,
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Datetime {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Datetime {
        #[inline]
        fn eq(&self, other: &Datetime) -> bool {
            self.date == other.date && self.time == other.time
                && self.offset == other.offset
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for Datetime {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<Option<Date>>;
            let _: ::core::cmp::AssertParamIsEq<Option<Time>>;
            let _: ::core::cmp::AssertParamIsEq<Option<Offset>>;
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for Datetime {
        #[inline]
        fn partial_cmp(
            &self,
            other: &Datetime,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            match ::core::cmp::PartialOrd::partial_cmp(&self.date, &other.date) {
                ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                    match ::core::cmp::PartialOrd::partial_cmp(&self.time, &other.time) {
                        ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                            ::core::cmp::PartialOrd::partial_cmp(
                                &self.offset,
                                &other.offset,
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
    impl ::core::cmp::Ord for Datetime {
        #[inline]
        fn cmp(&self, other: &Datetime) -> ::core::cmp::Ordering {
            match ::core::cmp::Ord::cmp(&self.date, &other.date) {
                ::core::cmp::Ordering::Equal => {
                    match ::core::cmp::Ord::cmp(&self.time, &other.time) {
                        ::core::cmp::Ordering::Equal => {
                            ::core::cmp::Ord::cmp(&self.offset, &other.offset)
                        }
                        cmp => cmp,
                    }
                }
                cmp => cmp,
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Datetime {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for Datetime {}
    #[automatically_derived]
    impl ::core::clone::Clone for Datetime {
        #[inline]
        fn clone(&self) -> Datetime {
            let _: ::core::clone::AssertParamIsClone<Option<Date>>;
            let _: ::core::clone::AssertParamIsClone<Option<Time>>;
            let _: ::core::clone::AssertParamIsClone<Option<Offset>>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Datetime {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Datetime",
                "date",
                &self.date,
                "time",
                &self.time,
                "offset",
                &&self.offset,
            )
        }
    }
    /// A parsed TOML date value
    ///
    /// May be part of a [`Datetime`]. Alone, `Date` corresponds to a [Local Date].
    /// From the TOML v1.0.0 spec:
    ///
    /// > If you include only the date portion of an RFC 3339 formatted date-time,
    /// > it will represent that entire day without any relation to an offset or
    /// > timezone.
    /// >
    /// > ```toml
    /// > ld1 = 1979-05-27
    /// > ```
    ///
    /// [Local Date]: https://toml.io/en/v1.0.0#local-date
    pub struct Date {
        /// Year: four digits
        pub year: u16,
        /// Month: 1 to 12
        pub month: u8,
        /// Day: 1 to {28, 29, 30, 31} (based on month/year)
        pub day: u8,
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Date {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Date {
        #[inline]
        fn eq(&self, other: &Date) -> bool {
            self.year == other.year && self.month == other.month && self.day == other.day
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for Date {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<u16>;
            let _: ::core::cmp::AssertParamIsEq<u8>;
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for Date {
        #[inline]
        fn partial_cmp(
            &self,
            other: &Date,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            match ::core::cmp::PartialOrd::partial_cmp(&self.year, &other.year) {
                ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                    match ::core::cmp::PartialOrd::partial_cmp(
                        &self.month,
                        &other.month,
                    ) {
                        ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                            ::core::cmp::PartialOrd::partial_cmp(&self.day, &other.day)
                        }
                        cmp => cmp,
                    }
                }
                cmp => cmp,
            }
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for Date {
        #[inline]
        fn cmp(&self, other: &Date) -> ::core::cmp::Ordering {
            match ::core::cmp::Ord::cmp(&self.year, &other.year) {
                ::core::cmp::Ordering::Equal => {
                    match ::core::cmp::Ord::cmp(&self.month, &other.month) {
                        ::core::cmp::Ordering::Equal => {
                            ::core::cmp::Ord::cmp(&self.day, &other.day)
                        }
                        cmp => cmp,
                    }
                }
                cmp => cmp,
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Date {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for Date {}
    #[automatically_derived]
    impl ::core::clone::Clone for Date {
        #[inline]
        fn clone(&self) -> Date {
            let _: ::core::clone::AssertParamIsClone<u16>;
            let _: ::core::clone::AssertParamIsClone<u8>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Date {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Date",
                "year",
                &self.year,
                "month",
                &self.month,
                "day",
                &&self.day,
            )
        }
    }
    /// A parsed TOML time value
    ///
    /// May be part of a [`Datetime`]. Alone, `Time` corresponds to a [Local Time].
    /// From the TOML v1.0.0 spec:
    ///
    /// > If you include only the time portion of an RFC 3339 formatted date-time,
    /// > it will represent that time of day without any relation to a specific
    /// > day or any offset or timezone.
    /// >
    /// > ```toml
    /// > lt1 = 07:32:00
    /// > lt2 = 00:32:00.999999
    /// > ```
    /// >
    /// > Millisecond precision is required. Further precision of fractional
    /// > seconds is implementation-specific. If the value contains greater
    /// > precision than the implementation can support, the additional precision
    /// > must be truncated, not rounded.
    ///
    /// [Local Time]: https://toml.io/en/v1.0.0#local-time
    pub struct Time {
        /// Hour: 0 to 23
        pub hour: u8,
        /// Minute: 0 to 59
        pub minute: u8,
        /// Second: 0 to {58, 59, 60} (based on leap second rules)
        pub second: Option<u8>,
        /// Nanosecond: 0 to `999_999_999`
        pub nanosecond: Option<u32>,
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Time {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Time {
        #[inline]
        fn eq(&self, other: &Time) -> bool {
            self.hour == other.hour && self.minute == other.minute
                && self.second == other.second && self.nanosecond == other.nanosecond
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for Time {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<u8>;
            let _: ::core::cmp::AssertParamIsEq<Option<u8>>;
            let _: ::core::cmp::AssertParamIsEq<Option<u32>>;
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for Time {
        #[inline]
        fn partial_cmp(
            &self,
            other: &Time,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            match ::core::cmp::PartialOrd::partial_cmp(&self.hour, &other.hour) {
                ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                    match ::core::cmp::PartialOrd::partial_cmp(
                        &self.minute,
                        &other.minute,
                    ) {
                        ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                            match ::core::cmp::PartialOrd::partial_cmp(
                                &self.second,
                                &other.second,
                            ) {
                                ::core::option::Option::Some(
                                    ::core::cmp::Ordering::Equal,
                                ) => {
                                    ::core::cmp::PartialOrd::partial_cmp(
                                        &self.nanosecond,
                                        &other.nanosecond,
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
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for Time {
        #[inline]
        fn cmp(&self, other: &Time) -> ::core::cmp::Ordering {
            match ::core::cmp::Ord::cmp(&self.hour, &other.hour) {
                ::core::cmp::Ordering::Equal => {
                    match ::core::cmp::Ord::cmp(&self.minute, &other.minute) {
                        ::core::cmp::Ordering::Equal => {
                            match ::core::cmp::Ord::cmp(&self.second, &other.second) {
                                ::core::cmp::Ordering::Equal => {
                                    ::core::cmp::Ord::cmp(&self.nanosecond, &other.nanosecond)
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
    impl ::core::marker::Copy for Time {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for Time {}
    #[automatically_derived]
    impl ::core::clone::Clone for Time {
        #[inline]
        fn clone(&self) -> Time {
            let _: ::core::clone::AssertParamIsClone<u8>;
            let _: ::core::clone::AssertParamIsClone<Option<u8>>;
            let _: ::core::clone::AssertParamIsClone<Option<u32>>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Time {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field4_finish(
                f,
                "Time",
                "hour",
                &self.hour,
                "minute",
                &self.minute,
                "second",
                &self.second,
                "nanosecond",
                &&self.nanosecond,
            )
        }
    }
    /// A parsed TOML time offset
    ///
    pub enum Offset {
        /// > A suffix which, when applied to a time, denotes a UTC offset of 00:00;
        /// > often spoken "Zulu" from the ICAO phonetic alphabet representation of
        /// > the letter "Z". --- [RFC 3339 section 2]
        ///
        /// [RFC 3339 section 2]: https://datatracker.ietf.org/doc/html/rfc3339#section-2
        Z,
        /// Offset between local time and UTC
        Custom {
            /// Minutes: -`1_440..1_440`
            minutes: i16,
        },
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Offset {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Offset {
        #[inline]
        fn eq(&self, other: &Offset) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
                && match (self, other) {
                    (
                        Offset::Custom { minutes: __self_0 },
                        Offset::Custom { minutes: __arg1_0 },
                    ) => __self_0 == __arg1_0,
                    _ => true,
                }
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for Offset {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<i16>;
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for Offset {
        #[inline]
        fn partial_cmp(
            &self,
            other: &Offset,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            match (self, other) {
                (
                    Offset::Custom { minutes: __self_0 },
                    Offset::Custom { minutes: __arg1_0 },
                ) => ::core::cmp::PartialOrd::partial_cmp(__self_0, __arg1_0),
                _ => ::core::cmp::PartialOrd::partial_cmp(&__self_discr, &__arg1_discr),
            }
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for Offset {
        #[inline]
        fn cmp(&self, other: &Offset) -> ::core::cmp::Ordering {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            match ::core::cmp::Ord::cmp(&__self_discr, &__arg1_discr) {
                ::core::cmp::Ordering::Equal => {
                    match (self, other) {
                        (
                            Offset::Custom { minutes: __self_0 },
                            Offset::Custom { minutes: __arg1_0 },
                        ) => ::core::cmp::Ord::cmp(__self_0, __arg1_0),
                        _ => ::core::cmp::Ordering::Equal,
                    }
                }
                cmp => cmp,
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Offset {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for Offset {}
    #[automatically_derived]
    impl ::core::clone::Clone for Offset {
        #[inline]
        fn clone(&self) -> Offset {
            let _: ::core::clone::AssertParamIsClone<i16>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Offset {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                Offset::Z => ::core::fmt::Formatter::write_str(f, "Z"),
                Offset::Custom { minutes: __self_0 } => {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "Custom",
                        "minutes",
                        &__self_0,
                    )
                }
            }
        }
    }
    impl Datetime {}
    impl Date {}
    impl Time {}
    impl From<Date> for Datetime {
        fn from(other: Date) -> Self {
            Self {
                date: Some(other),
                time: None,
                offset: None,
            }
        }
    }
    impl From<Time> for Datetime {
        fn from(other: Time) -> Self {
            Self {
                date: None,
                time: Some(other),
                offset: None,
            }
        }
    }
    impl fmt::Display for Datetime {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            if let Some(ref date) = self.date {
                f.write_fmt(format_args!("{0}", date))?;
            }
            if let Some(ref time) = self.time {
                if self.date.is_some() {
                    f.write_fmt(format_args!("T"))?;
                }
                f.write_fmt(format_args!("{0}", time))?;
            }
            if let Some(ref offset) = self.offset {
                f.write_fmt(format_args!("{0}", offset))?;
            }
            Ok(())
        }
    }
    impl fmt::Display for Date {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_fmt(
                format_args!("{0:04}-{1:02}-{2:02}", self.year, self.month, self.day),
            )
        }
    }
    impl fmt::Display for Time {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_fmt(format_args!("{0:02}:{1:02}", self.hour, self.minute))?;
            if let Some(second) = self
                .second
                .or_else(|| self.nanosecond.is_some().then_some(0))
            {
                f.write_fmt(format_args!(":{0:02}", second))?;
            }
            if let Some(nanosecond) = self.nanosecond {
                let s = ::alloc::__export::must_use({
                    ::alloc::fmt::format(format_args!("{0:09}", nanosecond))
                });
                let mut s = s.trim_end_matches('0');
                if s.is_empty() {
                    s = "0";
                }
                f.write_fmt(format_args!(".{0}", s))?;
            }
            Ok(())
        }
    }
    impl fmt::Display for Offset {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match *self {
                Self::Z => f.write_fmt(format_args!("Z")),
                Self::Custom { mut minutes } => {
                    let mut sign = '+';
                    if minutes < 0 {
                        minutes *= -1;
                        sign = '-';
                    }
                    let hours = minutes / 60;
                    let minutes = minutes % 60;
                    f.write_fmt(format_args!("{0}{1:02}:{2:02}", sign, hours, minutes))
                }
            }
        }
    }
    impl FromStr for Datetime {
        type Err = DatetimeParseError;
        fn from_str(date: &str) -> Result<Self, DatetimeParseError> {
            let mut result = Self {
                date: None,
                time: None,
                offset: None,
            };
            let mut lexer = Lexer::new(date);
            let digits = lexer
                .next()
                .ok_or(DatetimeParseError::new().expected("year or hour"))?;
            digits.is(TokenKind::Digits).map_err(|err| err.expected("year or hour"))?;
            let sep = lexer
                .next()
                .ok_or(
                    DatetimeParseError::new().expected("`-` (YYYY-MM) or `:` (HH:MM)"),
                )?;
            match sep.kind {
                TokenKind::Dash => {
                    let year = digits;
                    let month = lexer
                        .next()
                        .ok_or_else(|| {
                            DatetimeParseError::new().what("date").expected("month")
                        })?;
                    month
                        .is(TokenKind::Digits)
                        .map_err(|err| err.what("date").expected("month"))?;
                    let sep = lexer
                        .next()
                        .ok_or(
                            DatetimeParseError::new()
                                .what("date")
                                .expected("`-` (MM-DD)"),
                        )?;
                    sep.is(TokenKind::Dash)
                        .map_err(|err| err.what("date").expected("`-` (MM-DD)"))?;
                    let day = lexer
                        .next()
                        .ok_or(DatetimeParseError::new().what("date").expected("day"))?;
                    day.is(TokenKind::Digits)
                        .map_err(|err| err.what("date").expected("day"))?;
                    if year.raw.len() != 4 {
                        return Err(
                            DatetimeParseError::new()
                                .what("date")
                                .expected("a four-digit year (YYYY)"),
                        );
                    }
                    if month.raw.len() != 2 {
                        return Err(
                            DatetimeParseError::new()
                                .what("date")
                                .expected("a two-digit month (MM)"),
                        );
                    }
                    if day.raw.len() != 2 {
                        return Err(
                            DatetimeParseError::new()
                                .what("date")
                                .expected("a two-digit day (DD)"),
                        );
                    }
                    let date = Date {
                        year: year
                            .raw
                            .parse()
                            .map_err(|_err| DatetimeParseError::new())?,
                        month: month
                            .raw
                            .parse()
                            .map_err(|_err| DatetimeParseError::new())?,
                        day: day.raw.parse().map_err(|_err| DatetimeParseError::new())?,
                    };
                    if date.month < 1 || date.month > 12 {
                        return Err(
                            DatetimeParseError::new()
                                .what("date")
                                .expected("month between 01 and 12"),
                        );
                    }
                    let is_leap_year = (date.year % 4 == 0)
                        && ((date.year % 100 != 0) || (date.year % 400 == 0));
                    let (max_days_in_month, expected_day) = match date.month {
                        2 if is_leap_year => (29, "day between 01 and 29"),
                        2 => (28, "day between 01 and 28"),
                        4 | 6 | 9 | 11 => (30, "day between 01 and 30"),
                        _ => (31, "day between 01 and 31"),
                    };
                    if date.day < 1 || date.day > max_days_in_month {
                        return Err(
                            DatetimeParseError::new().what("date").expected(expected_day),
                        );
                    }
                    result.date = Some(date);
                }
                TokenKind::Colon => lexer = Lexer::new(date),
                _ => {
                    return Err(
                        DatetimeParseError::new()
                            .expected("`-` (YYYY-MM) or `:` (HH:MM)"),
                    );
                }
            }
            let partial_time = if result.date.is_some() {
                let sep = lexer.next();
                match sep {
                    Some(token) if #[allow(non_exhaustive_omitted_patterns)]
                    match token.kind {
                        TokenKind::T | TokenKind::Space => true,
                        _ => false,
                    } => true,
                    Some(_token) => {
                        return Err(
                            DatetimeParseError::new()
                                .what("date-time")
                                .expected("`T` between date and time"),
                        );
                    }
                    None => false,
                }
            } else {
                result.date.is_none()
            };
            if partial_time {
                let hour = lexer
                    .next()
                    .ok_or_else(|| {
                        DatetimeParseError::new().what("time").expected("hour")
                    })?;
                hour.is(TokenKind::Digits)
                    .map_err(|err| err.what("time").expected("hour"))?;
                let sep = lexer
                    .next()
                    .ok_or(
                        DatetimeParseError::new().what("time").expected("`:` (HH:MM)"),
                    )?;
                sep.is(TokenKind::Colon)
                    .map_err(|err| err.what("time").expected("`:` (HH:MM)"))?;
                let minute = lexer
                    .next()
                    .ok_or(DatetimeParseError::new().what("time").expected("minute"))?;
                minute
                    .is(TokenKind::Digits)
                    .map_err(|err| err.what("time").expected("minute"))?;
                let second = if lexer.clone().next().map(|t| t.kind)
                    == Some(TokenKind::Colon)
                {
                    let sep = lexer.next().ok_or(DatetimeParseError::new())?;
                    sep.is(TokenKind::Colon)?;
                    let second = lexer
                        .next()
                        .ok_or(
                            DatetimeParseError::new().what("time").expected("second"),
                        )?;
                    second
                        .is(TokenKind::Digits)
                        .map_err(|err| err.what("time").expected("second"))?;
                    Some(second)
                } else {
                    None
                };
                let nanosecond = if second.is_some()
                    && lexer.clone().next().map(|t| t.kind) == Some(TokenKind::Dot)
                {
                    let sep = lexer.next().ok_or(DatetimeParseError::new())?;
                    sep.is(TokenKind::Dot)?;
                    let nanosecond = lexer
                        .next()
                        .ok_or(
                            DatetimeParseError::new().what("time").expected("nanosecond"),
                        )?;
                    nanosecond
                        .is(TokenKind::Digits)
                        .map_err(|err| err.what("time").expected("nanosecond"))?;
                    Some(nanosecond)
                } else {
                    None
                };
                if hour.raw.len() != 2 {
                    return Err(
                        DatetimeParseError::new()
                            .what("time")
                            .expected("a two-digit hour (HH)"),
                    );
                }
                if minute.raw.len() != 2 {
                    return Err(
                        DatetimeParseError::new()
                            .what("time")
                            .expected("a two-digit minute (MM)"),
                    );
                }
                if let Some(second) = second {
                    if second.raw.len() != 2 {
                        return Err(
                            DatetimeParseError::new()
                                .what("time")
                                .expected("a two-digit second (SS)"),
                        );
                    }
                }
                let time = Time {
                    hour: hour.raw.parse().map_err(|_err| DatetimeParseError::new())?,
                    minute: minute
                        .raw
                        .parse()
                        .map_err(|_err| DatetimeParseError::new())?,
                    second: second
                        .map(|t| t.raw.parse().map_err(|_err| DatetimeParseError::new()))
                        .transpose()?,
                    nanosecond: nanosecond.map(|t| s_to_nanoseconds(t.raw)),
                };
                if time.hour > 23 {
                    return Err(
                        DatetimeParseError::new()
                            .what("time")
                            .expected("hour between 00 and 23"),
                    );
                }
                if time.minute > 59 {
                    return Err(
                        DatetimeParseError::new()
                            .what("time")
                            .expected("minute between 00 and 59"),
                    );
                }
                if time.second.unwrap_or(0) > 60 {
                    return Err(
                        DatetimeParseError::new()
                            .what("time")
                            .expected("second between 00 and 60"),
                    );
                }
                if time.nanosecond.unwrap_or(0) > 999_999_999 {
                    return Err(
                        DatetimeParseError::new()
                            .what("time")
                            .expected("nanoseconds overflowed"),
                    );
                }
                result.time = Some(time);
            }
            if result.date.is_some() && result.time.is_some() {
                match lexer.next() {
                    Some(token) if token.kind == TokenKind::Z => {
                        result.offset = Some(Offset::Z);
                    }
                    Some(token) if #[allow(non_exhaustive_omitted_patterns)]
                    match token.kind {
                        TokenKind::Plus | TokenKind::Dash => true,
                        _ => false,
                    } => {
                        let sign = if token.kind == TokenKind::Plus { 1 } else { -1 };
                        let hours = lexer
                            .next()
                            .ok_or(
                                DatetimeParseError::new().what("offset").expected("hour"),
                            )?;
                        hours
                            .is(TokenKind::Digits)
                            .map_err(|err| err.what("offset").expected("hour"))?;
                        let sep = lexer
                            .next()
                            .ok_or(
                                DatetimeParseError::new()
                                    .what("offset")
                                    .expected("`:` (HH:MM)"),
                            )?;
                        sep.is(TokenKind::Colon)
                            .map_err(|err| err.what("offset").expected("`:` (HH:MM)"))?;
                        let minutes = lexer
                            .next()
                            .ok_or(
                                DatetimeParseError::new().what("offset").expected("minute"),
                            )?;
                        minutes
                            .is(TokenKind::Digits)
                            .map_err(|err| err.what("offset").expected("minute"))?;
                        if hours.raw.len() != 2 {
                            return Err(
                                DatetimeParseError::new()
                                    .what("offset")
                                    .expected("a two-digit hour (HH)"),
                            );
                        }
                        if minutes.raw.len() != 2 {
                            return Err(
                                DatetimeParseError::new()
                                    .what("offset")
                                    .expected("a two-digit minute (MM)"),
                            );
                        }
                        let hours = hours
                            .raw
                            .parse::<u8>()
                            .map_err(|_err| DatetimeParseError::new())?;
                        let minutes = minutes
                            .raw
                            .parse::<u8>()
                            .map_err(|_err| DatetimeParseError::new())?;
                        if hours > 23 {
                            return Err(
                                DatetimeParseError::new()
                                    .what("offset")
                                    .expected("hours between 00 and 23"),
                            );
                        }
                        if minutes > 59 {
                            return Err(
                                DatetimeParseError::new()
                                    .what("offset")
                                    .expected("minutes between 00 and 59"),
                            );
                        }
                        let total_minutes = sign * (hours as i16 * 60 + minutes as i16);
                        if !((-24 * 60)..=(24 * 60)).contains(&total_minutes) {
                            return Err(DatetimeParseError::new().what("offset"));
                        }
                        result.offset = Some(Offset::Custom {
                            minutes: total_minutes,
                        });
                    }
                    Some(_token) => {
                        return Err(
                            DatetimeParseError::new()
                                .what("offset")
                                .expected("`Z`, +OFFSET, -OFFSET"),
                        );
                    }
                    None => {}
                }
            }
            if lexer.unknown().is_some() {
                return Err(DatetimeParseError::new());
            }
            Ok(result)
        }
    }
    fn s_to_nanoseconds(input: &str) -> u32 {
        let mut nanosecond = 0;
        for (i, byte) in input.bytes().enumerate() {
            if byte.is_ascii_digit() {
                if i < 9 {
                    let p = 10_u32.pow(8 - i as u32);
                    nanosecond += p * u32::from(byte - b'0');
                }
            } else {
                {
                    ::core::panicking::panic_fmt(
                        format_args!("invalid nanoseconds {0:?}", input),
                    );
                };
            }
        }
        nanosecond
    }
    struct Token<'s> {
        kind: TokenKind,
        raw: &'s str,
    }
    #[automatically_derived]
    impl<'s> ::core::marker::Copy for Token<'s> {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl<'s> ::core::clone::TrivialClone for Token<'s> {}
    #[automatically_derived]
    impl<'s> ::core::clone::Clone for Token<'s> {
        #[inline]
        fn clone(&self) -> Token<'s> {
            let _: ::core::clone::AssertParamIsClone<TokenKind>;
            let _: ::core::clone::AssertParamIsClone<&'s str>;
            *self
        }
    }
    impl Token<'_> {
        fn is(&self, kind: TokenKind) -> Result<(), DatetimeParseError> {
            if self.kind == kind { Ok(()) } else { Err(DatetimeParseError::new()) }
        }
    }
    enum TokenKind {
        Digits,
        Dash,
        Colon,
        Dot,
        T,
        Space,
        Z,
        Plus,
        Unknown,
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
    struct Lexer<'s> {
        stream: &'s str,
    }
    #[automatically_derived]
    impl<'s> ::core::marker::Copy for Lexer<'s> {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl<'s> ::core::clone::TrivialClone for Lexer<'s> {}
    #[automatically_derived]
    impl<'s> ::core::clone::Clone for Lexer<'s> {
        #[inline]
        fn clone(&self) -> Lexer<'s> {
            let _: ::core::clone::AssertParamIsClone<&'s str>;
            *self
        }
    }
    impl<'s> Lexer<'s> {
        fn new(input: &'s str) -> Self {
            Self { stream: input }
        }
        fn unknown(&mut self) -> Option<Token<'s>> {
            let remaining = self.stream.len();
            if remaining == 0 {
                return None;
            }
            let raw = self.stream;
            self.stream = &self.stream[remaining..remaining];
            Some(Token {
                kind: TokenKind::Unknown,
                raw,
            })
        }
    }
    impl<'s> Iterator for Lexer<'s> {
        type Item = Token<'s>;
        fn next(&mut self) -> Option<Self::Item> {
            let (kind, end) = match self.stream.as_bytes().first()? {
                b'0'..=b'9' => {
                    let end = self
                        .stream
                        .as_bytes()
                        .iter()
                        .position(|b| !b.is_ascii_digit())
                        .unwrap_or(self.stream.len());
                    (TokenKind::Digits, end)
                }
                b'-' => (TokenKind::Dash, 1),
                b':' => (TokenKind::Colon, 1),
                b'T' | b't' => (TokenKind::T, 1),
                b' ' => (TokenKind::Space, 1),
                b'Z' | b'z' => (TokenKind::Z, 1),
                b'+' => (TokenKind::Plus, 1),
                b'.' => (TokenKind::Dot, 1),
                _ => (TokenKind::Unknown, self.stream.len()),
            };
            let (raw, rest) = self.stream.split_at(end);
            self.stream = rest;
            Some(Token { kind, raw })
        }
    }
    /// Error returned from parsing a `Datetime` in the `FromStr` implementation.
    #[non_exhaustive]
    pub struct DatetimeParseError {
        what: Option<&'static str>,
        expected: Option<&'static str>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for DatetimeParseError {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "DatetimeParseError",
                "what",
                &self.what,
                "expected",
                &&self.expected,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for DatetimeParseError {
        #[inline]
        fn clone(&self) -> DatetimeParseError {
            DatetimeParseError {
                what: ::core::clone::Clone::clone(&self.what),
                expected: ::core::clone::Clone::clone(&self.expected),
            }
        }
    }
    impl DatetimeParseError {
        fn new() -> Self {
            Self { what: None, expected: None }
        }
        fn what(mut self, what: &'static str) -> Self {
            self.what = Some(what);
            self
        }
        fn expected(mut self, expected: &'static str) -> Self {
            self.expected = Some(expected);
            self
        }
    }
    impl fmt::Display for DatetimeParseError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            if let Some(what) = self.what {
                f.write_fmt(format_args!("invalid {0}", what))?;
            } else {
                "invalid datetime".fmt(f)?;
            }
            if let Some(expected) = self.expected {
                f.write_fmt(format_args!(", expected {0}", expected))?;
            }
            Ok(())
        }
    }
    impl core::error::Error for DatetimeParseError {}
}
pub use crate::datetime::Date;
pub use crate::datetime::Datetime;
pub use crate::datetime::DatetimeParseError;
pub use crate::datetime::Offset;
pub use crate::datetime::Time;
