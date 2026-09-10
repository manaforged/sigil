//! Single-keystroke string parser used by [`super::key::Key::parse`].
//!
//! Split out of `key.rs` to keep that file under the
//! 400-line file cap. No behavioural difference — the same `parse_tokens`
//! / `parse_code` / `parse_function_key` helpers, lifted verbatim.

use super::key::{Key, KeyCode, Modifier, Modifiers, NamedKey, ParseError};

pub(super) fn parse_tokens(lower: &str) -> Result<Key, ParseError> {
    let tokens: Vec<&str> = lower.split('-').collect();
    let mut modifiers = Modifiers::empty();
    let mut code: Option<KeyCode> = None;
    for (i, tok) in tokens.iter().enumerate() {
        let is_last = i + 1 == tokens.len();
        if let Some(m) = Modifier::parse(tok) {
            if modifiers.contains(m) {
                return Err(ParseError::DuplicateModifier { modifier: m });
            }
            modifiers = modifiers.with(m);
            continue;
        }
        // Not a modifier — must be the final key-code token. If a code
        // has already been claimed, the input has two trailing keys.
        if code.is_some() {
            return Err(ParseError::TooManyKeys);
        }
        if !is_last {
            // Mid-string non-modifier: the would-be modifier slot is
            // not a recognised modifier name.
            return Err(ParseError::UnknownToken {
                token: (*tok).to_string(),
            });
        }
        code = Some(parse_code(tok)?);
    }
    let code = code.ok_or(ParseError::NoKey)?;
    Ok(Key::new(modifiers, code))
}

fn parse_code(tok: &str) -> Result<KeyCode, ParseError> {
    if tok.is_empty() {
        return Err(ParseError::NoKey);
    }
    if let Some(named) = NamedKey::parse(tok) {
        return Ok(KeyCode::Named(named));
    }
    if let Some(n) = parse_function_key(tok)? {
        return Ok(KeyCode::Function(n));
    }
    let mut chars = tok.chars();
    let first = chars.next().ok_or(ParseError::NoKey)?;
    if chars.next().is_some() {
        return Err(ParseError::UnknownToken {
            token: tok.to_string(),
        });
    }
    if first.is_ascii_alphabetic() {
        return Ok(KeyCode::Char(first.to_ascii_lowercase()));
    }
    if let Some(d) = first.to_digit(10) {
        // `to_digit(10)` returns 0..=9 so the `as u8` is lossless.
        #[allow(clippy::cast_possible_truncation)]
        return Ok(KeyCode::Digit(d as u8));
    }
    if is_symbol(first) {
        return Ok(KeyCode::Symbol(first));
    }
    Err(ParseError::UnknownToken {
        token: tok.to_string(),
    })
}

fn parse_function_key(tok: &str) -> Result<Option<u8>, ParseError> {
    let Some(rest) = tok.strip_prefix('f') else {
        return Ok(None);
    };
    // `rest` may legitimately not be digits (e.g. `foo`) — fall back
    // to "not a function key" so the symbol/letter path can run.
    if rest.is_empty() {
        return Ok(None);
    }
    let n: u32 = match rest.parse() {
        Ok(n) => n,
        Err(_) => return Ok(None),
    };
    if !(1..=24).contains(&n) {
        return Err(ParseError::FunctionKeyOutOfRange { n });
    }
    // n is bounded 1..=24 — fits in u8.
    #[allow(clippy::cast_possible_truncation)]
    Ok(Some(n as u8))
}

const fn is_symbol(c: char) -> bool {
    matches!(
        c,
        ',' | '.'
            | '/'
            | ';'
            | '\''
            | '['
            | ']'
            | '\\'
            | '`'
            | '='
            | '-'
            | '~'
            | '!'
            | '@'
            | '#'
            | '$'
            | '%'
            | '^'
            | '&'
            | '*'
            | '('
            | ')'
            | '_'
            | '+'
            | '<'
            | '>'
            | '?'
            | ':'
            | '"'
            | '{'
            | '}'
            | '|'
    )
}
