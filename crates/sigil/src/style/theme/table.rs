use std::collections::HashMap;

use super::ThemeValue;
use crate::style::{Color, ColorAtom, Token};

/// One layer: a fixed array per `Token` plus a map for custom names.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TokenLayer {
    tokens: [Option<Color>; Token::COUNT],
    custom: HashMap<String, ThemeValue>,
}

impl Default for TokenLayer {
    fn default() -> Self {
        Self {
            tokens: [None; Token::COUNT],
            custom: HashMap::new(),
        }
    }
}

impl TokenLayer {
    fn token(&self, token: Token) -> Option<Color> {
        self.tokens.get(token.index()).copied().flatten()
    }

    /// Returns the color of `key`, if set.
    #[must_use]
    pub(crate) fn color(&self, key: ColorAtom) -> Option<Color> {
        match key {
            ColorAtom::Color(color) => Some(color),
            ColorAtom::Token(token) => self.token(token),
            ColorAtom::Custom(name) => match self.custom.get(name) {
                Some(ThemeValue::Color(color)) => Some(*color),
                _ => None,
            },
        }
    }

    /// Returns the value of the token `name`, if set.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn value(&self, name: &str) -> Option<ThemeValue> {
        Token::from_name(name).map_or_else(
            || self.custom.get(name).copied(),
            |token| self.token(token).map(ThemeValue::Color),
        )
    }

    /// Stores a token value and returns the old value.
    pub(crate) fn insert(&mut self, name: &str, value: ThemeValue) -> Option<ThemeValue> {
        let slot = Token::from_name(name).and_then(|token| self.tokens.get_mut(token.index()));
        match (slot, value) {
            (Some(slot), ThemeValue::Color(color)) => slot.replace(color).map(ThemeValue::Color),
            _ => self.custom.insert(name.to_owned(), value),
        }
    }

    /// Returns the names.
    #[cfg(debug_assertions)]
    pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
        Token::ALL
            .iter()
            .filter(|token| self.token(**token).is_some())
            .map(|token| token.name())
            .chain(self.custom.keys().map(String::as_str))
    }
}

/// The runtime override and theme layers of `Theme::tokens`.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TokenTable {
    /// The overrides.
    pub(crate) overrides: TokenLayer,
    /// The theme.
    pub(crate) theme: TokenLayer,
}
