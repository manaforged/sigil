use crate::reactive::Reactive;
use std::ops::RangeInclusive;
use std::rc::Rc;

use crate::kernel::{Cx, IntoNode, NodeId};
use crate::reactive::{Signal, derived, effect, signal};
use crate::style::locale::Locale;
use crate::style::theme::ambient::ActiveTheme;

use super::text_field::{TextFieldBuilder, text_field};

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

pub struct NumberFieldBuilder<T: NumericKind> {
    inner: TextFieldBuilder,
    buffer: Signal<String>,
    range: Rc<RangeInclusive<T>>,
    invalid: Option<Signal<bool>>,
    theme: ActiveTheme,
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
        let Self {
            inner,
            buffer,
            range,
            invalid,
            theme,
        } = self;
        inner.invalid_when(derived(move || {
            invalid.is_some_and(|flag| flag.get())
                || !in_range(&buffer.get(), &range, &theme.locale())
        }))
    }
}

fn in_range<T: NumericKind>(raw: &str, range: &RangeInclusive<T>, locale: &Locale) -> bool {
    T::parse(raw, locale).is_some_and(|parsed| {
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
    let theme = ActiveTheme::capture();
    let buffer = signal(value.peek().render(&theme.locale()));
    let range = Rc::new(range);

    let range_for_value = Rc::clone(&range);
    effect(move || {
        let clamped = value.get().clamp_to(&range_for_value);
        let rendered = clamped.render(&theme.locale());
        if buffer.peek().as_str() != rendered {
            buffer.set(rendered);
        }
    });

    let revert = {
        let range = Rc::clone(&range);
        Rc::new(move || {
            let locale = theme.locale();
            if T::parse(&buffer.peek(), &locale).is_none() {
                buffer.set(value.peek().clamp_to(&range).render(&locale));
            }
        })
    };
    let on_blur = Rc::clone(&revert);
    let inner = text_field(buffer)
        .change({
            let range = Rc::clone(&range);
            move |raw| {
                let Some(parsed) = T::parse(raw, &theme.locale()) else {
                    return;
                };
                let clamped = parsed.clamp_to(&range);
                let current = value.peek();
                if current.partial_cmp(&clamped) != Some(core::cmp::Ordering::Equal) {
                    value.set(clamped);
                }
            }
        })
        .submit(move |_| revert())
        .blur(move || on_blur());

    NumberFieldBuilder {
        inner,
        buffer,
        range,
        invalid: None,
        theme,
    }
}

#[cfg(test)]
#[path = "number_field_tests.rs"]
mod tests;
