use std::rc::Rc;

use crate::reactive::{Reactive, Signal, effect, memo, signal};
use crate::style::Theme;
use crate::style::theme::ambient::ActiveTheme;
use crate::widgets::Widget;

use super::super::text_field::{TextFieldBuilder, text_field};

pub(crate) trait FieldParser: 'static {
    type Value: Clone + PartialEq + 'static;
    const REVERT: bool = false;
    fn parse(&self, raw: &str, theme: &Theme) -> Option<Self::Value>;
    fn render(&self, value: &Self::Value, theme: &Theme) -> String;
    fn normalize(&self, value: Self::Value) -> Self::Value {
        value
    }
    fn accepts(&self, raw: &str, theme: &Theme) -> bool {
        self.parse(raw, theme).is_some()
    }
}

pub(crate) struct ParsedField<P: FieldParser> {
    inner: TextFieldBuilder,
    buffer: Signal<String>,
    parser: Rc<P>,
    disabled: Option<Signal<bool>>,
    theme: ActiveTheme,
}

impl<P: FieldParser> core::fmt::Debug for ParsedField<P> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ParsedField")
            .field("inner", &self.inner)
            .field("buffer", &self.buffer)
            .field("disabled", &self.disabled)
            .finish_non_exhaustive()
    }
}

impl<P: FieldParser> ParsedField<P> {
    pub(crate) fn new(value: Signal<P::Value>, parser: P) -> Self {
        let theme = ActiveTheme::capture();
        let parser = Rc::new(parser);
        let buffer = signal(theme.with(|current| shown(parser.as_ref(), value.peek(), current)));
        follow(value, buffer, Rc::clone(&parser), theme);
        let changed = Rc::clone(&parser);
        let mut inner = text_field(buffer).change(move |raw| {
            let Some(parsed) = theme.with(|current| changed.parse(raw, current)) else {
                return;
            };
            let parsed = changed.normalize(parsed);
            if value.peek() != parsed {
                value.set(parsed);
            }
        });
        let tidy = Rc::new({
            let parser = Rc::clone(&parser);
            move || {
                let settled = theme.with(|current| {
                    let valid = parser.parse(&buffer.peek(), current).is_some();
                    (valid || P::REVERT).then(|| shown(parser.as_ref(), value.peek(), current))
                });
                if let Some(settled) = settled.filter(|settled| *settled != buffer.peek()) {
                    buffer.set(settled);
                }
            }
        });
        let on_blur = Rc::clone(&tidy);
        inner = inner.submit(move |_| tidy()).focus_out(move || on_blur());
        Self {
            inner,
            buffer,
            parser,
            disabled: None,
            theme,
        }
    }

    pub(crate) fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        let disabled = disabled.into_signal();
        self.inner = self.inner.disabled(disabled);
        self.disabled = Some(disabled);
        self
    }

    pub(crate) const fn disabled_flag(&self) -> Option<Signal<bool>> {
        self.disabled
    }

    pub(crate) fn map(mut self, f: impl FnOnce(TextFieldBuilder) -> TextFieldBuilder) -> Self {
        self.inner = f(self.inner);
        self
    }

    pub(crate) fn invalid(self, invalid: impl Reactive<bool>) -> Self {
        self.map(move |inner| inner.invalid(invalid))
    }

    pub(crate) fn text_field(self) -> TextFieldBuilder {
        let Self {
            inner,
            buffer,
            parser,
            theme,
            ..
        } = self;
        inner.invalid(memo(move || {
            !theme.with(|current| parser.accepts(&buffer.get(), current))
        }))
    }
}

fn shown<P: FieldParser>(parser: &P, raw: P::Value, theme: &Theme) -> String {
    parser.render(&parser.normalize(raw), theme)
}

fn follow<P: FieldParser>(
    value: Signal<P::Value>,
    buffer: Signal<String>,
    parser: Rc<P>,
    theme: ActiveTheme,
) {
    let mut last = None;
    effect(move || {
        let current = parser.normalize(value.get());
        let typed = buffer.peek();
        let rendered = theme.with(|active| {
            let typed_now = last.as_ref() != Some(&current)
                && parser
                    .parse(&typed, active)
                    .is_some_and(|parsed| parser.normalize(parsed) == current);
            (!typed_now).then(|| parser.render(&current, active))
        });
        last = Some(current);
        if let Some(rendered) = rendered.filter(|rendered| *rendered != typed) {
            buffer.set(rendered);
        }
    });
}

macro_rules! parsed_controls {
    ([$($g:tt)*] $ty:ty) => {
        impl<$($g)*> $ty {
            /// Disables the field while `disabled` is true.
            pub fn disabled(mut self, disabled: impl $crate::reactive::Reactive<bool>) -> Self {
                self.field = self.field.disabled(disabled);
                self
            }

            /// Marks the field invalid while `invalid` is true, in paint and for screen readers; each call adds a condition.
            pub fn invalid(mut self, invalid: impl $crate::reactive::Reactive<bool>) -> Self {
                self.field = self.field.invalid(invalid);
                self
            }
        }

        impl<$($g)*> $crate::widgets::field::field_sealed::Sealed for $ty {}

        impl<$($g)*> $crate::widgets::FieldControl for $ty {
            fn invalid(self, invalid: impl $crate::reactive::Reactive<bool>) -> Self {
                Self::invalid(self, invalid)
            }
        }
    };
    ([$($g:tt)*] $ty:ty, text_field) => {
        $crate::widgets::field::parsed_controls!([$($g)*] $ty);

        impl<$($g)*> $ty {
            /// Returns the text field this input edits through.
            pub fn text_field(self) -> $crate::widgets::TextFieldBuilder {
                self.field.text_field()
            }
        }

        impl<$($g)*> $crate::kernel::IntoNode for $ty {
            fn build(self, cx: &mut $crate::kernel::Cx) -> $crate::kernel::NodeId {
                $crate::kernel::IntoNode::build(self.text_field(), cx)
            }
        }
    };
}

pub(crate) use parsed_controls;
