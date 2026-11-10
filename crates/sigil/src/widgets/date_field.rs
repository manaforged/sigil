use crate::kernel::{Cx, IntoNode, NodeId};
use crate::reactive::{Reactive, Signal};
use crate::style::locale::Locale;

use super::field::{FieldParser, ParsedField};
use super::text_field::TextFieldBuilder;

/// A calendar date.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Date {
    /// The year.
    pub year: i32,
    /// The month.
    pub month: u8,
    /// The day.
    pub day: u8,
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

    /// Creates a date.
    #[must_use]
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        if !(1..=12).contains(&month) {
            return None;
        }
        (1..=days_in_month(year, month))
            .contains(&day)
            .then_some(Self { year, month, day })
    }

    /// Returns the render.
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

    fn parse(&self, raw: &str, locale: &Locale) -> Option<Date> {
        Date::parse_in(raw, locale)
    }

    fn render(&self, value: &Date, locale: &Locale) -> String {
        value.render_in(locale)
    }
}

/// Builder of a date input.
#[derive(Debug)]
pub struct DateFieldBuilder {
    field: ParsedField<DateParser>,
}

impl DateFieldBuilder {
    /// Sets the disabled.
    #[must_use]
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.field = self.field.disabled(disabled);
        self
    }

    /// Sets the invalid when.
    #[must_use]
    pub fn invalid_when(mut self, invalid: impl Reactive<bool>) -> Self {
        self.field = self.field.invalid_when(invalid);
        self
    }

    /// Returns the text field.
    #[must_use]
    pub fn text_field(self) -> TextFieldBuilder {
        self.field.text_field()
    }
}

impl IntoNode for DateFieldBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.text_field().build(cx)
    }
}

/// A date input.
#[must_use]
pub fn date_field(value: Signal<Date>) -> DateFieldBuilder {
    DateFieldBuilder {
        field: ParsedField::new(value, DateParser),
    }
}

builder_impls!([] DateFieldBuilder: forward(field));

#[cfg(test)]
#[path = "date_field_tests.rs"]
mod tests;
