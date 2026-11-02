use std::ops::RangeInclusive;

use crate::kernel::{Cx, IntoNode, NodeId};
use crate::reactive::{Reactive, Signal};
use crate::style::locale::Locale;

use super::field::{FieldParser, ParsedField};
use super::text_field::TextFieldBuilder;

/// Number types that `number_field` accepts.
pub trait NumericKind: Copy + PartialOrd + 'static + private::Sealed {
    /// Parses localized text as this number type.
    #[must_use]
    fn parse(input: &str, locale: &Locale) -> Option<Self>;
    /// Formats this number as localized text.
    #[must_use]
    fn render(self, locale: &Locale) -> String;
    /// Returns this number kept inside the range.
    #[must_use]
    fn clamp_to(self, range: &RangeInclusive<Self>) -> Self;
}

mod private {
    pub trait Sealed {}
    impl Sealed for i32 {}
    impl Sealed for i64 {}
    impl Sealed for f32 {}
    impl Sealed for f64 {}
    impl Sealed for usize {}
}

macro_rules! impl_numeric {
    ($keep:expr => $($t:ty),+) => {$(
        impl NumericKind for $t {
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
    )+};
}

impl_numeric!(|_: &Self| true => i32, i64, usize);
impl_numeric!(|v: &Self| v.is_finite() => f32, f64);

struct NumberParser<T>(RangeInclusive<T>);

impl<T: NumericKind> FieldParser for NumberParser<T> {
    type Value = T;
    const REVERT: bool = true;

    fn parse(&self, raw: &str, locale: &Locale) -> Option<T> {
        T::parse(raw, locale)
    }

    fn render(&self, value: &T, locale: &Locale) -> String {
        value.render(locale)
    }

    fn normalize(&self, value: T) -> T {
        value.clamp_to(&self.0)
    }

    fn accepts(&self, raw: &str, locale: &Locale) -> bool {
        T::parse(raw, locale).is_some_and(|parsed| parsed == parsed.clamp_to(&self.0))
    }
}

/// Builder of a numeric input.
pub struct NumberFieldBuilder<T: NumericKind> {
    field: ParsedField<NumberParser<T>>,
}

impl<T: NumericKind> NumberFieldBuilder<T> {
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
    pub fn text_field(self) -> TextFieldBuilder {
        self.field.text_field()
    }
}

impl<T: NumericKind> IntoNode for NumberFieldBuilder<T> {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.text_field().build(cx)
    }
}

/// A numeric input.
#[must_use]
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
