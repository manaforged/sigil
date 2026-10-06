use core::str::FromStr;

use super::{KEY_LABELS, Key, LogicalKey, MODIFIER_LABELS, ModifiersState, Shortcut};

const MODIFIER_ALIASES: [(ModifiersState, &str); 8] = [
    (ModifiersState::CONTROL, "control"),
    (ModifiersState::ALT, "option"),
    (ModifiersState::ALT, "opt"),
    (ModifiersState::META, "cmd"),
    (ModifiersState::META, "command"),
    (ModifiersState::META, "meta"),
    (ModifiersState::META, "win"),
    (ModifiersState::CONTROL, "ctl"),
];

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ShortcutParseError {
    Empty,
    UnknownModifier(String),
    UnknownKey(String),
}

impl core::fmt::Display for ShortcutParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Empty => f.write_str("empty shortcut"),
            Self::UnknownModifier(name) => write!(f, "unknown modifier `{name}`"),
            Self::UnknownKey(name) => write!(f, "unknown key `{name}`"),
        }
    }
}

impl core::error::Error for ShortcutParseError {}

impl FromStr for Shortcut {
    type Err = ShortcutParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        if text.is_empty() {
            return Err(ShortcutParseError::Empty);
        }
        if text.contains('+') && text.len() > 1 {
            parse_joined(text)
        } else {
            parse_symbols(text)
        }
    }
}

fn parse_joined(text: &str) -> Result<Shortcut, ShortcutParseError> {
    let (head, key) = match text.rsplit_once('+') {
        Some((head, "")) => (head.strip_suffix('+').unwrap_or(head), "+"),
        Some(split) => split,
        None => ("", text),
    };
    let mut modifiers = ModifiersState::empty();
    for part in head
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let flag = modifier_named(part)
            .ok_or_else(|| ShortcutParseError::UnknownModifier(part.to_owned()))?;
        modifiers |= flag;
    }
    Ok(Shortcut::new(key_named(key.trim())?, modifiers))
}

fn parse_symbols(text: &str) -> Result<Shortcut, ShortcutParseError> {
    let mut modifiers = ModifiersState::empty();
    let mut rest = text;
    while let Some((flag, tail)) = MODIFIER_LABELS
        .iter()
        .find_map(|(flag, apple, _)| rest.strip_prefix(apple).map(|tail| (*flag, tail)))
        .filter(|(_, tail)| !tail.is_empty())
    {
        modifiers |= flag;
        rest = tail;
    }
    Ok(Shortcut::new(key_named(rest)?, modifiers))
}

fn modifier_named(name: &str) -> Option<ModifiersState> {
    MODIFIER_LABELS
        .iter()
        .find(|(_, apple, other)| *apple == name || other.eq_ignore_ascii_case(name))
        .map(|(flag, _, _)| *flag)
        .or_else(|| {
            MODIFIER_ALIASES
                .iter()
                .find(|(_, alias)| alias.eq_ignore_ascii_case(name))
                .map(|(flag, _)| *flag)
        })
}

fn key_named(name: &str) -> Result<LogicalKey, ShortcutParseError> {
    if name.is_empty() {
        return Err(ShortcutParseError::Empty);
    }
    if let Some((key, _, _)) = KEY_LABELS
        .iter()
        .find(|(_, apple, other)| *apple == name || other.eq_ignore_ascii_case(name))
    {
        return Ok(LogicalKey::Named(*key));
    }
    let mut chars = name.chars();
    if let (Some(single), None) = (chars.next(), chars.next()) {
        return Ok(LogicalKey::character(
            single.to_lowercase().collect::<String>(),
        ));
    }
    Key::from_str(name)
        .ok()
        .filter(|key| !matches!(key, Key::Unidentified))
        .or_else(|| named_case_insensitive(name))
        .map(LogicalKey::Named)
        .ok_or_else(|| ShortcutParseError::UnknownKey(name.to_owned()))
}

fn named_case_insensitive(name: &str) -> Option<Key> {
    let mut chars = name.chars();
    let first = chars.next()?.to_uppercase().collect::<String>();
    let candidate = first + &chars.as_str().to_ascii_lowercase();
    Key::from_str(&candidate)
        .ok()
        .filter(|key| !matches!(key, Key::Unidentified))
}
