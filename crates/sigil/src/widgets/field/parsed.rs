use std::rc::Rc;

use crate::reactive::{Reactive, Signal, derived, effect, signal};
use crate::style::locale::Locale;
use crate::style::theme::ambient::ActiveTheme;

use super::super::text_field::{TextFieldBuilder, text_field};

pub(crate) trait FieldParser: 'static {
    type Value: Clone + PartialEq + 'static;
    const REVERT: bool = false;
    fn parse(&self, raw: &str, locale: &Locale) -> Option<Self::Value>;
    fn render(&self, value: &Self::Value, locale: &Locale) -> String;
    fn normalize(&self, value: Self::Value) -> Self::Value {
        value
    }
    fn accepts(&self, raw: &str, locale: &Locale) -> bool {
        self.parse(raw, locale).is_some()
    }
}

pub(crate) struct ParsedField<P: FieldParser> {
    inner: TextFieldBuilder,
    buffer: Signal<String>,
    parser: Rc<P>,
    invalid: Option<Signal<bool>>,
    theme: ActiveTheme,
}

impl<P: FieldParser> core::fmt::Debug for ParsedField<P> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ParsedField")
            .field("inner", &self.inner)
            .field("buffer", &self.buffer)
            .field("invalid", &self.invalid)
            .finish_non_exhaustive()
    }
}

impl<P: FieldParser> ParsedField<P> {
    pub(crate) fn new(value: Signal<P::Value>, parser: P) -> Self {
        let theme = ActiveTheme::capture();
        let parser = Rc::new(parser);
        let shown =
            |using: &P, raw: P::Value, locale: &Locale| using.render(&using.normalize(raw), locale);
        let buffer = signal(shown(&parser, value.peek(), &theme.locale()));
        let outward = Rc::clone(&parser);
        effect(move || {
            let rendered = shown(&outward, value.get(), &theme.locale());
            if buffer.peek().as_str() != rendered {
                buffer.set(rendered);
            }
        });
        let changed = Rc::clone(&parser);
        let mut inner = text_field(buffer).change(move |raw| {
            let Some(parsed) = changed.parse(raw, &theme.locale()) else {
                return;
            };
            let parsed = changed.normalize(parsed);
            if value.peek() != parsed {
                value.set(parsed);
            }
        });
        if P::REVERT {
            let revert = Rc::new({
                let parser = Rc::clone(&parser);
                move || {
                    let locale = theme.locale();
                    if parser.parse(&buffer.peek(), &locale).is_none() {
                        buffer.set(shown(&parser, value.peek(), &locale));
                    }
                }
            });
            let on_blur = Rc::clone(&revert);
            inner = inner.submit(move |_| revert()).blur(move || on_blur());
        }
        Self {
            inner,
            buffer,
            parser,
            invalid: None,
            theme,
        }
    }

    pub(crate) fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.inner = self.inner.disabled(disabled);
        self
    }

    pub(crate) fn map(mut self, f: impl FnOnce(TextFieldBuilder) -> TextFieldBuilder) -> Self {
        self.inner = f(self.inner);
        self
    }

    pub(crate) fn invalid_when(mut self, invalid: impl Reactive<bool>) -> Self {
        self.invalid = Some(invalid.into_signal());
        self
    }

    pub(crate) fn text_field(self) -> TextFieldBuilder {
        let Self {
            inner,
            buffer,
            parser,
            invalid,
            theme,
        } = self;
        inner.invalid_when(derived(move || {
            invalid.is_some_and(|flag| flag.get())
                || !parser.accepts(&buffer.get(), &theme.locale())
        }))
    }
}
