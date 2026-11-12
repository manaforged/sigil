use std::cell::Cell;
use std::fmt::Display;
use std::sync::OnceLock;

use crate::reactive::{Signal, memo, untrack};
use crate::style::strings::StringKey;

use super::Theme;

thread_local! {
    static ACTIVE: Cell<Option<Signal<Theme>>> = const { Cell::new(None) };
}

pub(crate) fn swap_active_theme(theme: Option<Signal<Theme>>) -> Option<Signal<Theme>> {
    ACTIVE.replace(theme)
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ActiveTheme {
    source: Option<Signal<Theme>>,
}

impl ActiveTheme {
    pub(crate) fn capture() -> Self {
        Self {
            source: ACTIVE.with(Cell::get),
        }
    }

    pub(crate) fn with<R>(self, read: impl FnOnce(&Theme) -> R) -> R {
        match self.source {
            Some(theme) => theme.with(read),
            None => read(fallback_theme()),
        }
    }

    pub(crate) fn string(self, key: StringKey) -> String {
        self.with(|theme| theme.strings.get(key).to_owned())
    }

    pub(crate) fn fill(self, key: StringKey, args: &[&dyn Display]) -> String {
        self.with(|theme| theme.strings.fill(key, args))
    }

    pub(crate) fn text(self, key: StringKey) -> impl Fn() -> String + 'static {
        move || self.string(key)
    }

    pub(crate) fn label(self, key: StringKey) -> Signal<String> {
        memo(self.text(key))
    }
}

pub(crate) fn fallback_theme<'a>() -> &'a Theme {
    static DEFAULT: OnceLock<Theme> = OnceLock::new();
    DEFAULT.get_or_init(Theme::default)
}

pub(crate) fn theme_string(key: StringKey) -> String {
    ActiveTheme::capture().string(key)
}

pub(crate) fn theme_string_once(key: StringKey) -> String {
    untrack(|| theme_string(key))
}

pub(crate) fn theme_fill_once(key: StringKey, args: &[&dyn Display]) -> String {
    untrack(|| ActiveTheme::capture().fill(key, args))
}

pub(crate) fn theme_label(key: StringKey) -> Signal<String> {
    ActiveTheme::capture().label(key)
}

pub(crate) fn theme_text(key: StringKey) -> impl Fn() -> String + 'static {
    ActiveTheme::capture().text(key)
}
