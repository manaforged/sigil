use std::fmt::Write as _;

use crate::kernel::{Cx, IntoNode, NodeId};
use crate::reactive::{Signal, effect, signal};

use super::text_field::{TextFieldBuilder, text_field};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl Date {
    #[must_use]
    pub fn parse(input: &str) -> Option<Self> {
        let mut parts = input.trim().splitn(3, '-');
        let year = parts.next()?.parse::<i32>().ok()?;
        let month = parts.next()?.parse::<u8>().ok()?;
        let day = parts.next()?.parse::<u8>().ok()?;
        Self::new(year, month, day)
    }

    #[must_use]
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        if !(1..=12).contains(&month) {
            return None;
        }
        let max = days_in_month(year, month);
        if day == 0 || day > max {
            return None;
        }
        Some(Self { year, month, day })
    }

    #[must_use]
    pub fn render(self) -> String {
        let mut out = String::with_capacity(10);
        let _ = write!(out, "{:04}-{:02}-{:02}", self.year, self.month, self.day);
        out
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

pub struct DateFieldBuilder {
    inner: TextFieldBuilder,
}

impl DateFieldBuilder {
    pub fn text_field(self) -> TextFieldBuilder {
        self.inner
    }
}

impl IntoNode for DateFieldBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.inner.build(cx)
    }
}

#[must_use]
pub fn date_field(value: Signal<Date>) -> DateFieldBuilder {
    let buffer = signal(value.peek().render());
    let buf_for_value = buffer;
    effect(move || {
        let rendered = value.get().render();
        if buf_for_value.peek().as_str() != rendered {
            buf_for_value.set(rendered);
        }
    });
    let inner = text_field(buffer).change(move |raw| {
        let Some(parsed) = Date::parse(raw) else {
            return;
        };
        if value.peek() != parsed {
            value.set(parsed);
        }
    });
    DateFieldBuilder { inner }
}

#[cfg(test)]
#[path = "date_field_tests.rs"]
mod tests;
