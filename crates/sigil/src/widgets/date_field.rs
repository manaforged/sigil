use crate::reactive::Reactive;

use crate::kernel::{Cx, IntoNode, NodeId};
use crate::reactive::{Signal, derived, effect, signal};
use crate::style::locale::Locale;
use crate::style::theme::ambient::ActiveTheme;

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
        let max = days_in_month(year, month);
        if day == 0 || day > max {
            return None;
        }
        Some(Self { year, month, day })
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

pub struct DateFieldBuilder {
    inner: TextFieldBuilder,
    buffer: Signal<String>,
    invalid: Option<Signal<bool>>,
    theme: ActiveTheme,
}

impl DateFieldBuilder {
    #[must_use]
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.inner = self.inner.disabled(disabled);
        self
    }

    #[must_use]
    pub fn invalid_when(mut self, invalid: impl crate::reactive::Reactive<bool>) -> Self {
        self.invalid = Some(invalid.into_signal());
        self
    }

    pub fn text_field(self) -> TextFieldBuilder {
        let buffer = self.buffer;
        let invalid = self.invalid;
        let theme = self.theme;
        self.inner.invalid_when(derived(move || {
            invalid.is_some_and(|flag| flag.get())
                || Date::parse_in(&buffer.get(), &theme.locale()).is_none()
        }))
    }
}

impl IntoNode for DateFieldBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.text_field().build(cx)
    }
}

#[must_use]
pub fn date_field(value: Signal<Date>) -> DateFieldBuilder {
    let theme = ActiveTheme::capture();
    let buffer = signal(value.peek().render_in(&theme.locale()));
    let buf_for_value = buffer;
    effect(move || {
        let rendered = value.get().render_in(&theme.locale());
        if buf_for_value.peek().as_str() != rendered {
            buf_for_value.set(rendered);
        }
    });
    let inner = text_field(buffer).change(move |raw| {
        let Some(parsed) = Date::parse_in(raw, &theme.locale()) else {
            return;
        };
        if value.peek() != parsed {
            value.set(parsed);
        }
    });
    DateFieldBuilder {
        inner,
        buffer,
        invalid: None,
        theme,
    }
}

#[cfg(test)]
#[path = "date_field_tests.rs"]
mod tests;
