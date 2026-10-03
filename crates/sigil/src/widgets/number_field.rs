use crate::reactive::Reactive;
use std::ops::RangeInclusive;
use std::rc::Rc;

use crate::kernel::{Cx, IntoNode, NodeId};
use crate::reactive::{Signal, derived, effect, signal};

use super::text_field::{TextFieldBuilder, text_field};

pub trait NumericKind: Copy + PartialOrd + 'static + private::Sealed {
    #[must_use]
    fn parse(input: &str) -> Option<Self>;
    #[must_use]
    fn render(self) -> String;
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

macro_rules! impl_numeric_int {
    ($t:ty) => {
        impl NumericKind for $t {
            fn parse(input: &str) -> Option<Self> {
                input.trim().parse::<Self>().ok()
            }
            fn render(self) -> String {
                self.to_string()
            }
            fn clamp_to(self, range: &RangeInclusive<Self>) -> Self {
                self.clamp(*range.start(), *range.end())
            }
        }
    };
}

impl_numeric_int!(i32);
impl_numeric_int!(i64);
impl_numeric_int!(usize);

macro_rules! impl_numeric_float {
    ($t:ty) => {
        impl NumericKind for $t {
            fn parse(input: &str) -> Option<Self> {
                input.trim().parse::<Self>().ok().filter(|v| v.is_finite())
            }
            fn render(self) -> String {
                self.to_string()
            }
            fn clamp_to(self, range: &RangeInclusive<Self>) -> Self {
                self.clamp(*range.start(), *range.end())
            }
        }
    };
}

impl_numeric_float!(f32);
impl_numeric_float!(f64);

pub struct NumberFieldBuilder<T: NumericKind> {
    inner: TextFieldBuilder,
    buffer: Signal<String>,
    range: Rc<RangeInclusive<T>>,
    invalid: Option<Signal<bool>>,
}

impl<T: NumericKind> NumberFieldBuilder<T> {
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
        let range = self.range;
        let invalid = self.invalid;
        self.inner.invalid_when(derived(move || {
            invalid.is_some_and(|flag| flag.get()) || !in_range(&buffer.get(), &range)
        }))
    }
}

fn in_range<T: NumericKind>(raw: &str, range: &RangeInclusive<T>) -> bool {
    T::parse(raw).is_some_and(|parsed| {
        parsed.partial_cmp(&parsed.clamp_to(range)) == Some(core::cmp::Ordering::Equal)
    })
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
    let buffer = signal(value.peek().render());
    let range = Rc::new(range);

    let buf_for_value = buffer;
    let range_for_value = Rc::clone(&range);
    effect(move || {
        let clamped = value.get().clamp_to(&range_for_value);
        let rendered = clamped.render();
        if buf_for_value.peek().as_str() != rendered {
            buf_for_value.set(rendered);
        }
    });

    let inner = text_field(buffer).change({
        let range = Rc::clone(&range);
        move |raw| {
            let Some(parsed) = T::parse(raw) else {
                return;
            };
            let clamped = parsed.clamp_to(&range);
            let current = value.peek();
            if current.partial_cmp(&clamped) != Some(core::cmp::Ordering::Equal) {
                value.set(clamped);
            }
        }
    });

    NumberFieldBuilder {
        inner,
        buffer,
        range,
        invalid: None,
    }
}

#[cfg(test)]
#[path = "number_field_tests.rs"]
mod tests;
