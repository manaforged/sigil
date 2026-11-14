use std::collections::HashMap;

use crate::style::{Color, ColorValue, Token};

/// One layer: a fixed array per `Token` plus a map for custom names.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TokenLayer {
    tokens: [Option<Color>; Token::COUNT],
    custom: HashMap<String, Color>,
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
        token.index().map_or_else(
            || self.custom.get(token.name()).copied(),
            |index| self.tokens.get(index).copied().flatten(),
        )
    }

    /// Returns the color of `key`, if set.
    #[must_use]
    pub(crate) fn color(&self, key: ColorValue) -> Option<Color> {
        match key {
            ColorValue::Color(color) => Some(color),
            ColorValue::Token(token) => self.token(token.canonical()),
            ColorValue::Custom(name) => Token::from_name(name)
                .map_or_else(|| self.custom.get(name).copied(), |token| self.token(token)),
        }
    }

    /// Stores the color of `token` and returns the old color.
    pub(crate) fn insert(&mut self, token: Token, color: Color) -> Option<Color> {
        match token.canonical().index() {
            Some(index) => self
                .tokens
                .get_mut(index)
                .and_then(|slot| slot.replace(color)),
            None => self.custom.insert(token.name().to_owned(), color),
        }
    }

    /// Stores the color of the token `name` and returns the old color.
    pub(crate) fn insert_name(&mut self, name: &str, color: Color) -> Option<Color> {
        match Token::from_name(name) {
            Some(token) => self.insert(token, color),
            None => self.custom.insert(name.to_owned(), color),
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
    /// The values set at run time on top of the theme.
    pub(crate) overrides: TokenLayer,
    /// The values that the theme defines.
    pub(crate) theme: TokenLayer,
}
