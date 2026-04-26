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
    pub(crate) const FIELD: &str = "$__toml_private_datetime";
    pub(crate) const NAME: &str = "$__toml_private_Datetime";
    pub(crate) fn is_datetime(name: &'static str) -> bool {
        name == NAME
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
    impl Datetime {
        fn type_name(&self) -> &'static str {
            match (self.date.is_some(), self.time.is_some(), self.offset.is_some()) {
                (true, true, true) => "offset datetime",
                (true, true, false) => "local datetime",
                (true, false, false) => Date::type_name(),
                (false, true, false) => Time::type_name(),
                _ => {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "internal error: entered unreachable code: {0}",
                            format_args!("unsupported datetime combination"),
                        ),
                    );
                }
            }
        }
    }
    impl Date {
        fn type_name() -> &'static str {
            "local date"
        }
    }
    impl Time {
        fn type_name() -> &'static str {
            "local time"
        }
    }
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
    impl serde_core::ser::Serialize for Datetime {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde_core::ser::Serializer,
        {
            use crate::alloc::string::ToString as _;
            use serde_core::ser::SerializeStruct;
            let mut s = serializer.serialize_struct(NAME, 1)?;
            s.serialize_field(FIELD, &self.to_string())?;
            s.end()
        }
    }
    impl serde_core::ser::Serialize for Date {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde_core::ser::Serializer,
        {
            Datetime::from(*self).serialize(serializer)
        }
    }
    impl serde_core::ser::Serialize for Time {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde_core::ser::Serializer,
        {
            Datetime::from(*self).serialize(serializer)
        }
    }
    impl<'de> serde_core::de::Deserialize<'de> for Datetime {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde_core::de::Deserializer<'de>,
        {
            struct DatetimeVisitor;
            impl<'de> serde_core::de::Visitor<'de> for DatetimeVisitor {
                type Value = Datetime;
                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter.write_str("a TOML datetime")
                }
                fn visit_map<V>(self, mut visitor: V) -> Result<Datetime, V::Error>
                where
                    V: serde_core::de::MapAccess<'de>,
                {
                    let value = visitor.next_key::<DatetimeKey>()?;
                    if value.is_none() {
                        return Err(
                            serde_core::de::Error::custom("datetime key not found"),
                        );
                    }
                    let v: DatetimeFromString = visitor.next_value()?;
                    Ok(v.value)
                }
            }
            static FIELDS: [&str; 1] = [FIELD];
            deserializer.deserialize_struct(NAME, &FIELDS, DatetimeVisitor)
        }
    }
    impl<'de> serde_core::de::Deserialize<'de> for Date {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde_core::de::Deserializer<'de>,
        {
            match Datetime::deserialize(deserializer)? {
                Datetime { date: Some(date), time: None, offset: None } => Ok(date),
                datetime => {
                    Err(
                        serde_core::de::Error::invalid_type(
                            serde_core::de::Unexpected::Other(datetime.type_name()),
                            &Self::type_name(),
                        ),
                    )
                }
            }
        }
    }
    impl<'de> serde_core::de::Deserialize<'de> for Time {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde_core::de::Deserializer<'de>,
        {
            match Datetime::deserialize(deserializer)? {
                Datetime { date: None, time: Some(time), offset: None } => Ok(time),
                datetime => {
                    Err(
                        serde_core::de::Error::invalid_type(
                            serde_core::de::Unexpected::Other(datetime.type_name()),
                            &Self::type_name(),
                        ),
                    )
                }
            }
        }
    }
    struct DatetimeKey;
    impl<'de> serde_core::de::Deserialize<'de> for DatetimeKey {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde_core::de::Deserializer<'de>,
        {
            struct FieldVisitor;
            impl serde_core::de::Visitor<'_> for FieldVisitor {
                type Value = ();
                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter.write_str("a valid datetime field")
                }
                fn visit_str<E>(self, s: &str) -> Result<(), E>
                where
                    E: serde_core::de::Error,
                {
                    if s == FIELD {
                        Ok(())
                    } else {
                        Err(
                            serde_core::de::Error::custom(
                                "expected field with custom name",
                            ),
                        )
                    }
                }
            }
            deserializer.deserialize_identifier(FieldVisitor)?;
            Ok(Self)
        }
    }
    pub(crate) struct DatetimeFromString {
        pub(crate) value: Datetime,
    }
    impl<'de> serde_core::de::Deserialize<'de> for DatetimeFromString {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde_core::de::Deserializer<'de>,
        {
            struct Visitor;
            impl serde_core::de::Visitor<'_> for Visitor {
                type Value = DatetimeFromString;
                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter.write_str("string containing a datetime")
                }
                fn visit_str<E>(self, s: &str) -> Result<DatetimeFromString, E>
                where
                    E: serde_core::de::Error,
                {
                    match s.parse() {
                        Ok(date) => Ok(DatetimeFromString { value: date }),
                        Err(e) => Err(serde_core::de::Error::custom(e)),
                    }
                }
            }
            deserializer.deserialize_str(Visitor)
        }
    }
}
pub mod de {
    //! Deserialization support for [`Datetime`][crate::Datetime]
    use alloc::string::ToString;
    use serde_core::de::IntoDeserializer;
    use serde_core::de::value::BorrowedStrDeserializer;
    /// Check if deserializing a [`Datetime`][crate::Datetime]
    pub fn is_datetime(name: &'static str) -> bool {
        crate::datetime::is_datetime(name)
    }
    /// Deserializer / format support for emitting [`Datetime`][crate::Datetime]
    pub struct DatetimeDeserializer<E> {
        date: Option<crate::Datetime>,
        _error: core::marker::PhantomData<E>,
    }
    impl<E> DatetimeDeserializer<E> {
        /// Create a deserializer to emit [`Datetime`][crate::Datetime]
        pub fn new(date: crate::Datetime) -> Self {
            Self {
                date: Some(date),
                _error: Default::default(),
            }
        }
    }
    impl<'de, E> serde_core::de::MapAccess<'de> for DatetimeDeserializer<E>
    where
        E: serde_core::de::Error,
    {
        type Error = E;
        fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
        where
            K: serde_core::de::DeserializeSeed<'de>,
        {
            if self.date.is_some() {
                seed.deserialize(BorrowedStrDeserializer::new(crate::datetime::FIELD))
                    .map(Some)
            } else {
                Ok(None)
            }
        }
        fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
        where
            V: serde_core::de::DeserializeSeed<'de>,
        {
            if let Some(date) = self.date.take() {
                seed.deserialize(date.to_string().into_deserializer())
            } else {
                {
                    ::core::panicking::panic_fmt(
                        format_args!("next_value_seed called before next_key_seed"),
                    );
                }
            }
        }
    }
    /// Integrate [`Datetime`][crate::Datetime] into an untagged deserialize
    pub enum VisitMap<'de> {
        /// The map was deserialized as a [Datetime][crate::Datetime] value
        Datetime(crate::Datetime),
        /// The map is of an unknown format and needs further deserialization
        Key(alloc::borrow::Cow<'de, str>),
    }
    impl<'de> VisitMap<'de> {
        /// Determine the type of the map by deserializing it
        pub fn next_key_seed<V: serde_core::de::MapAccess<'de>>(
            visitor: &mut V,
        ) -> Result<Option<Self>, V::Error> {
            let mut key = None;
            let Some(()) = visitor.next_key_seed(DatetimeOrTable::new(&mut key))? else {
                return Ok(None);
            };
            let result = if let Some(key) = key {
                VisitMap::Key(key)
            } else {
                let date: crate::datetime::DatetimeFromString = visitor.next_value()?;
                VisitMap::Datetime(date.value)
            };
            Ok(Some(result))
        }
    }
    struct DatetimeOrTable<'m, 'de> {
        key: &'m mut Option<alloc::borrow::Cow<'de, str>>,
    }
    impl<'m, 'de> DatetimeOrTable<'m, 'de> {
        fn new(key: &'m mut Option<alloc::borrow::Cow<'de, str>>) -> Self {
            *key = None;
            Self { key }
        }
    }
    impl<'de> serde_core::de::DeserializeSeed<'de> for DatetimeOrTable<'_, 'de> {
        type Value = ();
        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde_core::de::Deserializer<'de>,
        {
            deserializer.deserialize_any(self)
        }
    }
    impl<'de> serde_core::de::Visitor<'de> for DatetimeOrTable<'_, 'de> {
        type Value = ();
        fn expecting(
            &self,
            formatter: &mut core::fmt::Formatter<'_>,
        ) -> core::fmt::Result {
            formatter.write_str("a string key")
        }
        fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
        where
            E: serde_core::de::Error,
        {
            if s == crate::datetime::FIELD {
                *self.key = None;
                Ok(())
            } else {
                use crate::alloc::borrow::ToOwned as _;
                *self.key = Some(alloc::borrow::Cow::Owned(s.to_owned()));
                Ok(())
            }
        }
        fn visit_borrowed_str<E>(self, s: &'de str) -> Result<Self::Value, E>
        where
            E: serde_core::de::Error,
        {
            if s == crate::datetime::FIELD {
                *self.key = None;
                Ok(())
            } else {
                *self.key = Some(alloc::borrow::Cow::Borrowed(s));
                Ok(())
            }
        }
        #[allow(unused_qualifications)]
        fn visit_string<E>(self, s: alloc::string::String) -> Result<Self::Value, E>
        where
            E: serde_core::de::Error,
        {
            if s == crate::datetime::FIELD {
                *self.key = None;
                Ok(())
            } else {
                *self.key = Some(alloc::borrow::Cow::Owned(s));
                Ok(())
            }
        }
    }
}
pub mod ser {
    //! Serialization support for [`Datetime`][crate::Datetime]
    /// Check if serializing a [`Datetime`][crate::Datetime]
    pub fn is_datetime(name: &'static str) -> bool {
        crate::datetime::is_datetime(name)
    }
    /// See [`DatetimeSerializer`]
    #[non_exhaustive]
    pub enum SerializerError {
        /// Unsupported datetime format
        InvalidFormat(crate::DatetimeParseError),
        /// Unsupported serialization protocol
        InvalidProtocol,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for SerializerError {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                SerializerError::InvalidFormat(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "InvalidFormat",
                        &__self_0,
                    )
                }
                SerializerError::InvalidProtocol => {
                    ::core::fmt::Formatter::write_str(f, "InvalidProtocol")
                }
            }
        }
    }
    impl serde_core::ser::Error for SerializerError {
        fn custom<T>(_msg: T) -> Self
        where
            T: core::fmt::Display,
        {
            Self::InvalidProtocol
        }
    }
    impl core::fmt::Display for SerializerError {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::InvalidFormat(e) => e.fmt(formatter),
                Self::InvalidProtocol => "invalid serialization protocol".fmt(formatter),
            }
        }
    }
    impl core::error::Error for SerializerError {}
    /// Serializer / format support for emitting [`Datetime`][crate::Datetime]
    pub struct DatetimeSerializer {
        value: Option<crate::Datetime>,
    }
    #[automatically_derived]
    impl ::core::default::Default for DatetimeSerializer {
        #[inline]
        fn default() -> DatetimeSerializer {
            DatetimeSerializer {
                value: ::core::default::Default::default(),
            }
        }
    }
    impl DatetimeSerializer {
        /// Create a serializer to emit [`Datetime`][crate::Datetime]
        pub fn new() -> Self {
            Self { value: None }
        }
        /// See [`serde_core::ser::SerializeStruct::serialize_field`]
        pub fn serialize_field<T>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), SerializerError>
        where
            T: serde_core::ser::Serialize + ?Sized,
        {
            if key == crate::datetime::FIELD {
                self.value = Some(value.serialize(DatetimeFieldSerializer::default())?);
            }
            Ok(())
        }
        /// See [`serde_core::ser::SerializeStruct::end`]
        pub fn end(self) -> Result<crate::Datetime, SerializerError> {
            self.value.ok_or(SerializerError::InvalidProtocol)
        }
    }
    struct DatetimeFieldSerializer {}
    #[automatically_derived]
    impl ::core::default::Default for DatetimeFieldSerializer {
        #[inline]
        fn default() -> DatetimeFieldSerializer {
            DatetimeFieldSerializer {}
        }
    }
    impl serde_core::ser::Serializer for DatetimeFieldSerializer {
        type Ok = crate::Datetime;
        type Error = SerializerError;
        type SerializeSeq = serde_core::ser::Impossible<Self::Ok, Self::Error>;
        type SerializeTuple = serde_core::ser::Impossible<Self::Ok, Self::Error>;
        type SerializeTupleStruct = serde_core::ser::Impossible<Self::Ok, Self::Error>;
        type SerializeTupleVariant = serde_core::ser::Impossible<Self::Ok, Self::Error>;
        type SerializeMap = serde_core::ser::Impossible<Self::Ok, Self::Error>;
        type SerializeStruct = serde_core::ser::Impossible<Self::Ok, Self::Error>;
        type SerializeStructVariant = serde_core::ser::Impossible<Self::Ok, Self::Error>;
        fn serialize_bool(self, _value: bool) -> Result<Self::Ok, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_i8(self, _value: i8) -> Result<Self::Ok, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_i16(self, _value: i16) -> Result<Self::Ok, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_i32(self, _value: i32) -> Result<Self::Ok, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_i64(self, _value: i64) -> Result<Self::Ok, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_u8(self, _value: u8) -> Result<Self::Ok, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_u16(self, _value: u16) -> Result<Self::Ok, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_u32(self, _value: u32) -> Result<Self::Ok, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_u64(self, _value: u64) -> Result<Self::Ok, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_f32(self, _value: f32) -> Result<Self::Ok, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_f64(self, _value: f64) -> Result<Self::Ok, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_char(self, _value: char) -> Result<Self::Ok, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
            v.parse::<crate::Datetime>().map_err(SerializerError::InvalidFormat)
        }
        fn serialize_bytes(self, _value: &[u8]) -> Result<Self::Ok, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_some<T>(self, _value: &T) -> Result<Self::Ok, Self::Error>
        where
            T: serde_core::ser::Serialize + ?Sized,
        {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_unit_struct(
            self,
            _name: &'static str,
        ) -> Result<Self::Ok, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_unit_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
        ) -> Result<Self::Ok, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_newtype_struct<T>(
            self,
            _name: &'static str,
            _value: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: serde_core::ser::Serialize + ?Sized,
        {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_newtype_variant<T>(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _value: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: serde_core::ser::Serialize + ?Sized,
        {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_seq(
            self,
            _len: Option<usize>,
        ) -> Result<Self::SerializeSeq, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_tuple(
            self,
            _len: usize,
        ) -> Result<Self::SerializeTuple, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_tuple_struct(
            self,
            _name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleStruct, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_tuple_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleVariant, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_map(
            self,
            _len: Option<usize>,
        ) -> Result<Self::SerializeMap, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_struct(
            self,
            _name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStruct, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
        fn serialize_struct_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStructVariant, Self::Error> {
            Err(SerializerError::InvalidProtocol)
        }
    }
}
pub use crate::datetime::Date;
pub use crate::datetime::Datetime;
pub use crate::datetime::DatetimeParseError;
pub use crate::datetime::Offset;
pub use crate::datetime::Time;
