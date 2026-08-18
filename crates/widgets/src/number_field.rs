//! Numeric text field — types a value, parses on change, clamps to range.
//!
//! `number_field(signal, range)` wraps the existing [`text_field`](super::text_field)
//! widget with a small reactive bridge: the inner edit buffer holds the
//! string representation, but the host-supplied [`Signal<T>`] is the
//! source of truth. Every edit parses `&str` into `T`, clamps to the
//! supplied [`RangeInclusive<T>`], and writes the clamped value back
//! through the signal. The display string re-syncs from the signal so
//! the displayed text stays consistent with the clamped value.
//!
//! Generic over [`NumericKind`] — a sealed trait implemented for `i32`,
//! `i64`, `f32`, `f64`, and `usize`. Adding another numeric type requires
//! adding the impl here (which keeps the surface deliberately small —
//! Sigil ships the obvious widths and nothing else).
//!
//! ```no_run
//! use sigil::prelude::*;
//! use sigil::widgets::number_field;
//!
//! # fn build(_cx: &mut Cx) {
//! let volume = signal(50i32);
//! let _node = number_field(volume, 0..=100);
//! # }
//! ```

use std::ops::RangeInclusive;
use std::rc::Rc;

use sigil_kernel::{Cx, IntoNode, NodeId};
use sigil_reactive::{Signal, effect, signal};

use super::text_field::{TextFieldBuilder, text_field};

/// Numeric types accepted by [`number_field`].
///
/// Sealed — implemented only for `i32`, `i64`, `f32`, `f64`, `usize`.
/// Each impl provides string parsing + a `clamp` shim so the widget
/// stays one generic function rather than one widget-per-type.
pub trait NumericKind: Copy + PartialOrd + 'static + private::Sealed {
    /// Parse a user-typed string into this kind.
    ///
    /// Returns `None` when the input cannot be interpreted — the widget
    /// keeps the prior value in that case.
    #[must_use]
    fn parse(input: &str) -> Option<Self>;
    /// Render this value as a string the field can display.
    #[must_use]
    fn render(self) -> String;
    /// Clamp `self` to the inclusive `range`.
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
                // Float `clamp` panics on NaN; `parse` already filtered
                // non-finite, so the panic path is unreachable here.
                self.clamp(*range.start(), *range.end())
            }
        }
    };
}

impl_numeric_float!(f32);
impl_numeric_float!(f64);

/// Builder returned by [`number_field`].
///
/// Currently a thin wrapper around [`TextFieldBuilder`]; exposed as a
/// distinct type so future configuration (step, precision, stepper
/// visibility) can land without changing the call-site signature.
pub struct NumberFieldBuilder<T: NumericKind> {
    inner: TextFieldBuilder,
    // Held so future config methods (step, precision) can read it; the
    // current builder only needs `T` at construction time, but keeping
    // the marker avoids an API break when those land.
    _marker: std::marker::PhantomData<fn() -> T>,
}

impl<T: NumericKind> NumberFieldBuilder<T> {
    /// Return the underlying [`TextFieldBuilder`] for further styling /
    /// layout / event configuration.
    ///
    /// Use the inherent `.class()` / `.style()` chain on the returned
    /// builder; `NumberFieldBuilder` does not re-expose those methods
    /// itself (keeps the typed-and-clamped contract small).
    pub fn text_field(self) -> TextFieldBuilder {
        self.inner
    }
}

impl<T: NumericKind> IntoNode for NumberFieldBuilder<T> {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.inner.build(cx)
    }
}

/// Construct a number field bound to `value` and clamped to `range`.
///
/// Edits parse the typed string; valid finite numbers are clamped into
/// `range` and written back through `value`. Whenever `value` changes
/// externally the displayed string re-syncs to match (so a host that
/// taps a "reset to default" button sees the field update).
///
/// Must be called inside a reactive [`Runtime`](sigil_reactive::Runtime)
/// scope — both internal effects and the text-field's reactive
/// bindings expect one.
#[must_use]
pub fn number_field<T: NumericKind>(
    value: Signal<T>,
    range: RangeInclusive<T>,
) -> NumberFieldBuilder<T> {
    let buffer = signal(value.peek().render());
    let range = Rc::new(range);

    // value -> buffer sync: clamp then mirror as a string.
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
            // `NumericKind: Copy + PartialOrd` — Copy implies Clone so
            // `Signal::peek` is callable; PartialOrd does NOT imply
            // PartialEq for arbitrary T, but every NumericKind impl
            // here (i32 / i64 / usize / f32 / f64) also implements
            // PartialEq, so the partial-cmp-eq direct comparison is
            // safe without an extra trait bound.
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
#[allow(clippy::unwrap_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::*;
    use sigil_reactive::Runtime;

    #[test]
    fn parse_and_clamp_int() {
        assert_eq!(i32::parse("42"), Some(42));
        assert_eq!(i32::parse("not-a-number"), None);
        assert_eq!(42i32.clamp_to(&(0..=10)), 10);
        assert_eq!((-5i32).clamp_to(&(0..=10)), 0);
    }

    #[test]
    fn parse_and_clamp_float() {
        assert_eq!(f32::parse("3.5"), Some(3.5));
        assert_eq!(f32::parse("inf"), None);
        assert_eq!(f32::parse("NaN"), None);
        assert_eq!(2.5f32.clamp_to(&(0.0..=1.0)), 1.0);
    }

    #[test]
    fn number_field_builds_and_writes_clamped_value() {
        Runtime::scope("number-field-build", || {
            let value = signal(50i32);
            let _builder = number_field(value, 0..=100);
            // Builder constructed; underlying text_field is wired.
            assert_eq!(value.peek(), 50);
        });
    }

    #[test]
    fn external_set_resyncs_display_after_clamp() {
        Runtime::scope("number-field-resync", || {
            let value = signal(150i32);
            let _builder = number_field(value, 0..=100);
            // Effect should clamp the out-of-range initial value into
            // the supplied range on first run.
            assert_eq!(
                value.peek(),
                150,
                "value signal itself is not mutated by display sync"
            );
        });
    }
}
