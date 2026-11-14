use std::ops::RangeInclusive;

use crate::reactive::Signal;
use crate::style::Theme;
use crate::style::locale::Locale;

use super::field::{FieldParser, ParsedField, parsed_controls};

/// Number types that `number_field` accepts: every built-in integer and float type.
pub trait NumericKind: Copy + PartialOrd + 'static + private::Sealed {}

mod private {
    use super::{Locale, RangeInclusive};

    #[allow(unnameable_types, reason = "sealed")]
    pub trait Sealed: Copy + PartialOrd + 'static {
        #[must_use]
        fn parse(input: &str, locale: &Locale) -> Option<Self>;
        #[must_use]
        fn render(self, locale: &Locale) -> String;
        #[must_use]
        fn clamp_to(self, range: &RangeInclusive<Self>) -> Self;
    }
}

macro_rules! impl_numeric {
    ($keep:expr => $($t:ty),+) => {$(
        impl private::Sealed for $t {
            fn parse(input: &str, locale: &Locale) -> Option<Self> {
                locale.normalize_number(input)?.parse::<Self>().ok().filter($keep)
            }
            fn render(self, locale: &Locale) -> String {
                locale.localize_number(&self.to_string())
            }
            fn clamp_to(self, range: &RangeInclusive<Self>) -> Self {
                self.clamp(*range.start(), *range.end())
            }
        }

        impl NumericKind for $t {}
    )+};
}

impl_numeric!(|_: &Self| true => u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
impl_numeric!(|v: &Self| v.is_finite() => f32, f64);

struct NumberParser<T>(RangeInclusive<T>);

impl<T: NumericKind> FieldParser for NumberParser<T> {
    type Value = T;
    const REVERT: bool = true;

    fn parse(&self, raw: &str, theme: &Theme) -> Option<T> {
        <T as private::Sealed>::parse(raw, &theme.locale)
    }

    fn render(&self, value: &T, theme: &Theme) -> String {
        private::Sealed::render(*value, &theme.locale)
    }

    fn normalize(&self, value: T) -> T {
        private::Sealed::clamp_to(value, &self.0)
    }

    fn accepts(&self, raw: &str, theme: &Theme) -> bool {
        <T as private::Sealed>::parse(raw, &theme.locale)
            .is_some_and(|parsed| parsed == private::Sealed::clamp_to(parsed, &self.0))
    }
}

/// Builder of a numeric input.
#[derive(Debug)]
#[must_use]
pub struct NumberFieldBuilder<T: NumericKind> {
    field: ParsedField<NumberParser<T>>,
}

parsed_controls!([T: NumericKind] NumberFieldBuilder<T>, text_field);

/// A numeric input.
pub fn number_field<T: NumericKind>(
    value: Signal<T>,
    range: RangeInclusive<T>,
) -> NumberFieldBuilder<T> {
    NumberFieldBuilder {
        field: ParsedField::new(value, NumberParser(range)),
    }
}

builder_impls!([T: NumericKind] NumberFieldBuilder<T>: forward(field));

#[cfg(test)]
#[path = "number_field_tests.rs"]
mod tests;
