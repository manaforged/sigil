use crate::kernel::{Cx, IntoNode, NodeId};
use crate::reactive::{Reactive, Signal};
use crate::style::locale::Locale;

use super::field::{FieldParser, ParsedField};
use super::text_field::TextFieldBuilder;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl Date {
    #[must_use]
    pub fn parse(input: &str) -> Option<Self> {
        Self::parse_in(input, &Locale::default())
    }

    #[must_use]
    pub fn parse_in(input: &str, locale: &Locale) -> Option<Self> {
        let (year, month, day) = locale.parse_date(input)?;
        Self::new(year, month, day)
    }

    #[must_use]
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        if !(1..=12).contains(&month) {
            return None;
        }
        (1..=days_in_month(year, month))
            .contains(&day)
            .then_some(Self { year, month, day })
    }

    #[must_use]
    pub fn render(self) -> String {
        self.render_in(&Locale::default())
    }

    #[must_use]
    pub fn render_in(self, locale: &Locale) -> String {
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

pub struct DateFieldBuilder {
    field: ParsedField<DateParser>,
}

impl DateFieldBuilder {
    #[must_use]
    pub fn disabled(self, disabled: impl Reactive<bool>) -> Self {
        Self {
            field: self.field.disabled(disabled),
        }
    }

    #[must_use]
    pub fn invalid_when(self, invalid: impl Reactive<bool>) -> Self {
        Self {
            field: self.field.invalid_when(invalid),
        }
    }

    pub fn text_field(self) -> TextFieldBuilder {
        self.field.text_field()
    }
}

impl IntoNode for DateFieldBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.text_field().build(cx)
    }
}

#[must_use]
pub fn date_field(value: Signal<Date>) -> DateFieldBuilder {
    DateFieldBuilder {
        field: ParsedField::new(value, DateParser),
    }
}

#[cfg(test)]
#[path = "date_field_tests.rs"]
mod tests;
