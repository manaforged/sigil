use crate::reactive::{Signal, signal};
use crate::style::locale::Locale;
use crate::style::{Strings, Theme};

use super::date_field::Date;
use super::field::{FieldParser, ParsedField, parsed_controls};

/// A time of day.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct Time {
    /// The hour, from 0 to 23.
    pub(crate) hour: u8,
    /// The minute.
    pub(crate) minute: u8,
    /// The second.
    pub(crate) second: u8,
}

impl Time {
    /// The hour, from 0 to 23.
    #[must_use]
    pub const fn hour(self) -> u8 {
        self.hour
    }

    /// The minute, from 0 to 59.
    #[must_use]
    pub const fn minute(self) -> u8 {
        self.minute
    }

    /// The second, from 0 to 59.
    #[must_use]
    pub const fn second(self) -> u8 {
        self.second
    }

    /// Creates a time, or `None` when a part is out of range.
    #[must_use]
    pub const fn new(hour: u8, minute: u8, second: u8) -> Option<Self> {
        if hour < 24 && minute < 60 && second < 60 {
            Some(Self {
                hour,
                minute,
                second,
            })
        } else {
            None
        }
    }

    /// Parses a 24-hour time such as `14:30` or `14:30:05`.
    #[must_use]
    pub fn parse(input: &str) -> Option<Self> {
        Self::parse_in(input, &Locale::default(), &Strings::english())
    }

    /// Formats this time on a 24-hour clock with seconds.
    #[must_use]
    pub fn render(self) -> String {
        self.render_in(&Locale::default(), &Strings::english(), true)
    }

    fn parse_in(input: &str, locale: &Locale, strings: &Strings) -> Option<Self> {
        let (hour, minute, second) = locale.parse_time(input, strings)?;
        Self::new(hour, minute, second.unwrap_or(0))
    }

    fn render_in(self, locale: &Locale, strings: &Strings, seconds: bool) -> String {
        locale.format_time(
            self.hour,
            self.minute,
            seconds.then_some(self.second),
            strings,
        )
    }
}

/// A calendar date and a time of day.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct DateTime {
    /// The date.
    pub(crate) date: Date,
    /// The time of day.
    pub(crate) time: Time,
}

impl DateTime {
    /// The calendar date.
    #[must_use]
    pub const fn date(self) -> Date {
        self.date
    }

    /// The time of day.
    #[must_use]
    pub const fn time(self) -> Time {
        self.time
    }

    /// Creates a date and time.
    #[must_use]
    pub const fn new(date: Date, time: Time) -> Self {
        Self { date, time }
    }

    /// Parses an ISO date and a 24-hour time separated by a space or `T`, such as `2026-11-11 14:30`.
    #[must_use]
    pub fn parse(input: &str) -> Option<Self> {
        Self::parse_in(input, &Locale::default(), &Strings::english())
    }

    /// Formats this value as an ISO date and a 24-hour time with seconds.
    #[must_use]
    pub fn render(self) -> String {
        self.render_in(&Locale::default(), &Strings::english(), true)
    }

    fn parse_in(input: &str, locale: &Locale, strings: &Strings) -> Option<Self> {
        let (date, time) = input.trim().split_once([' ', 'T'])?;
        let (year, month, day) = locale.parse_date(date)?;
        Some(Self::new(
            Date::new(year, month, day)?,
            Time::parse_in(time, locale, strings)?,
        ))
    }

    fn render_in(self, locale: &Locale, strings: &Strings, seconds: bool) -> String {
        let Date { year, month, day } = self.date;
        format!(
            "{} {}",
            locale.format_date(year, month, day),
            self.time.render_in(locale, strings, seconds)
        )
    }
}

/// A value a time field edits: `Time` or `DateTime`.
pub trait TimeValue: Clone + PartialEq + 'static + sealed::Sealed {}

impl TimeValue for Time {}
impl TimeValue for DateTime {}

mod sealed {
    use crate::style::Theme;

    #[allow(unnameable_types, reason = "sealed")]
    pub trait Sealed: Sized {
        fn parse(input: &str, theme: &Theme) -> Option<Self>;
        fn render(&self, theme: &Theme, seconds: bool) -> String;
    }

    macro_rules! sealed {
        ($($ty:ty),*) => {$(
            impl Sealed for $ty {
                fn parse(input: &str, theme: &Theme) -> Option<Self> {
                    Self::parse_in(input, &theme.locale, &theme.strings)
                }

                fn render(&self, theme: &Theme, seconds: bool) -> String {
                    self.render_in(&theme.locale, &theme.strings, seconds)
                }
            }
        )*};
    }

    sealed!(super::Time, super::DateTime);
}

struct TimeParser<T> {
    seconds: Signal<bool>,
    value: core::marker::PhantomData<T>,
}

impl<T: TimeValue> FieldParser for TimeParser<T> {
    type Value = T;

    fn parse(&self, raw: &str, theme: &Theme) -> Option<T> {
        <T as sealed::Sealed>::parse(raw, theme)
    }

    fn render(&self, value: &T, theme: &Theme) -> String {
        sealed::Sealed::render(value, theme, self.seconds.get())
    }
}

/// Builder of a time input.
#[derive(Debug)]
#[must_use]
pub struct TimeFieldBuilder<T: TimeValue> {
    field: ParsedField<TimeParser<T>>,
    seconds: Signal<bool>,
}

impl<T: TimeValue> TimeFieldBuilder<T> {
    /// Shows seconds when true.
    pub fn seconds(self, seconds: bool) -> Self {
        self.seconds.set(seconds);
        self
    }
}

parsed_controls!([T: TimeValue] TimeFieldBuilder<T>, text_field);

/// A time input that edits a `Time`, or a date and time input that edits a `DateTime`, on the clock of the theme's `Locale`.
///
/// ```
/// use sigil::prelude::*;
/// use sigil::widgets::{Time, time_field};
///
/// fn alarm() -> impl IntoNode {
///     let at = signal(Time::new(7, 30, 0).unwrap_or_default());
///     field("Alarm", time_field(at).seconds(false))
/// }
/// ```
pub fn time_field<T: TimeValue>(value: Signal<T>) -> TimeFieldBuilder<T> {
    let seconds = signal(false);
    TimeFieldBuilder {
        field: ParsedField::new(
            value,
            TimeParser {
                seconds,
                value: core::marker::PhantomData,
            },
        ),
        seconds,
    }
}

builder_impls!([T: TimeValue] TimeFieldBuilder<T>: forward(field));
