use std::ops::RangeInclusive;
use std::rc::Rc;

use crate::kernel::{Cx, IntoNode, NodeId};
use crate::reactive::{Signal, effect, signal};

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
    _marker: std::marker::PhantomData<fn() -> T>,
}

impl<T: NumericKind> NumberFieldBuilder<T> {
    pub fn text_field(self) -> TextFieldBuilder {
        self.inner
    }
}

impl<T: NumericKind> IntoNode for NumberFieldBuilder<T> {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.inner.build(cx)
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
        _marker: std::marker::PhantomData,
    }
}

#[cfg(test)]
#[path = "number_field_tests.rs"]
mod tests;
