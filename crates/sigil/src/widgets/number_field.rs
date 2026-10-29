use std::ops::RangeInclusive;

use crate::kernel::{Cx, IntoNode, NodeId};
use crate::reactive::{Reactive, Signal};
use crate::style::locale::Locale;

use super::field::{FieldParser, ParsedField};
use super::text_field::TextFieldBuilder;

pub trait NumericKind: Copy + PartialOrd + 'static + private::Sealed {
    #[must_use]
    fn parse(input: &str, locale: &Locale) -> Option<Self>;
    #[must_use]
    fn render(self, locale: &Locale) -> String;
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

pub struct NumberFieldBuilder<T: NumericKind> {
    field: ParsedField<NumberParser<T>>,
}

impl<T: NumericKind> NumberFieldBuilder<T> {
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

impl<T: NumericKind> IntoNode for NumberFieldBuilder<T> {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.text_field().build(cx)
    }
}

#[must_use]
pub fn number_field<T: NumericKind>(
    value: Signal<T>,
    range: RangeInclusive<T>,
) -> NumberFieldBuilder<T> {
    NumberFieldBuilder {
        field: ParsedField::new(value, NumberParser(range)),
    }
}

#[cfg(test)]
#[path = "number_field_tests.rs"]
mod tests;
