use crate::reactive::Signal;
use crate::style::Theme;
use crate::style::locale::Locale;

use super::field::{FieldParser, ParsedField, parsed_controls};

/// A calendar date.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct Date {
    /// The year in the proleptic Gregorian calendar.
    pub(crate) year: i32,
    /// The month, from 1 to 12.
    pub(crate) month: u8,
    /// The day of the month, from 1.
    pub(crate) day: u8,
}

impl Date {
    /// Parses a date in ISO format.
    #[must_use]
    pub fn parse(input: &str) -> Option<Self> {
        Self::parse_in(input, &Locale::default())
    }

    /// Parses a date in the order of `locale`.
    #[must_use]
    fn parse_in(input: &str, locale: &Locale) -> Option<Self> {
        let (year, month, day) = locale.parse_date(input)?;
        Self::new(year, month, day)
    }

    /// Creates a date; `None` when `month` or `day` is not in the calendar for `year`.
    #[must_use]
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        if !(1..=12).contains(&month) {
            return None;
        }
        (1..=days_in_month(year, month))
            .contains(&day)
            .then_some(Self { year, month, day })
    }

    /// Returns the year in the proleptic Gregorian calendar.
    #[must_use]
    pub const fn year(self) -> i32 {
        self.year
    }

    /// The month, from 1 to 12.
    #[must_use]
    pub const fn month(self) -> u8 {
        self.month
    }

    /// The day of the month, from 1.
    #[must_use]
    pub const fn day(self) -> u8 {
        self.day
    }

    /// The date as text in the format of the default `Locale`.
    #[must_use]
    pub fn render(self) -> String {
        self.render_in(&Locale::default())
    }

    /// Formats this date in the order of `locale`.
    #[must_use]
    fn render_in(self, locale: &Locale) -> String {
        locale.format_date(self.year, self.month, self.day)
    }
}

const fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

const fn is_leap(year: i32) -> bool {
    let y = year.unsigned_abs();
    y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400))
}

struct DateParser;

impl FieldParser for DateParser {
    type Value = Date;

    fn parse(&self, raw: &str, theme: &Theme) -> Option<Date> {
        Date::parse_in(raw, &theme.locale)
    }

    fn render(&self, value: &Date, theme: &Theme) -> String {
        value.render_in(&theme.locale)
    }
}

/// Builder of a date input.
#[derive(Debug)]
#[must_use]
pub struct DateFieldBuilder {
    field: ParsedField<DateParser>,
}

parsed_controls!([] DateFieldBuilder, text_field);

/// A date input.
pub fn date_field(value: Signal<Date>) -> DateFieldBuilder {
    DateFieldBuilder {
        field: ParsedField::new(value, DateParser),
    }
}

builder_impls!([] DateFieldBuilder: forward(field));

#[cfg(test)]
#[path = "date_field_tests.rs"]
mod tests;
