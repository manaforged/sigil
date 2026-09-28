use std::collections::HashMap;

use super::ThemeValue;
use crate::style::{ColorKey, Rgba8, Token};

#[derive(Debug, Clone, PartialEq)]
pub struct TokenLayer {
    tokens: [Option<Rgba8>; Token::COUNT],
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
    fn token(&self, token: Token) -> Option<Rgba8> {
        self.tokens.get(token.index()).copied().flatten()
    }

    #[must_use]
    pub fn color(&self, key: ColorKey) -> Option<Rgba8> {
        match key {
            ColorKey::Token(token) => self.token(token),
            ColorKey::Custom(name) => match self.custom.get(name) {
                Some(ThemeValue::Color(color)) => Some(*color),
                _ => None,
            },
        }
    }

    #[must_use]
    pub fn value(&self, name: &str) -> Option<ThemeValue> {
        Token::from_name(name).map_or_else(
            || self.custom.get(name).copied(),
            |token| self.token(token).map(ThemeValue::Color),
        )
    }

    pub fn insert(&mut self, name: &str, value: ThemeValue) -> Option<ThemeValue> {
        let slot = Token::from_name(name).and_then(|token| self.tokens.get_mut(token.index()));
        match (slot, value) {
            (Some(slot), ThemeValue::Color(color)) => slot.replace(color).map(ThemeValue::Color),
            _ => self.custom.insert(name.to_owned(), value),
        }
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        Token::ALL
            .iter()
            .filter(|token| self.token(**token).is_some())
            .map(|token| token.name())
            .chain(self.custom.keys().map(String::as_str))
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TokenTable {
    pub overrides: TokenLayer,
    pub theme: TokenLayer,
}
