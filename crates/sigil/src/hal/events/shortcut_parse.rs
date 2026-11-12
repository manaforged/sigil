use core::str::FromStr;

use crate::{Error, ErrorKind};

use super::{KEY_LABELS, Key, LogicalKey, MODIFIER_LABELS, Modifiers, Shortcut};

const MODIFIER_ALIASES: [(Modifiers, &str); 8] = [
    (Modifiers::CTRL, "control"),
    (Modifiers::ALT, "option"),
    (Modifiers::ALT, "opt"),
    (Modifiers::META, "cmd"),
    (Modifiers::META, "command"),
    (Modifiers::META, "meta"),
    (Modifiers::META, "win"),
    (Modifiers::CTRL, "ctl"),
];

impl FromStr for Shortcut {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        if text.is_empty() {
            return Err(Error::fixed(ErrorKind::EmptyShortcut, "empty shortcut"));
        }
        if text.contains('+') && text.len() > 1 {
            parse_joined(text)
        } else {
            parse_symbols(text)
        }
    }
}

fn parse_joined(text: &str) -> Result<Shortcut, Error> {
    let (head, key) = match text.rsplit_once('+') {
        Some((head, "")) => (head.strip_suffix('+').unwrap_or(head), "+"),
        Some(split) => split,
        None => ("", text),
    };
    let mut modifiers = Modifiers::empty();
    for part in head
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let flag = modifier_named(part).ok_or_else(|| {
            Error::message(
                ErrorKind::UnknownModifier,
                format_args!("unknown modifier `{part}`"),
            )
        })?;
        modifiers |= flag;
    }
    Ok(Shortcut::new(key_named(key.trim())?, modifiers))
}

fn parse_symbols(text: &str) -> Result<Shortcut, Error> {
    let mut modifiers = Modifiers::empty();
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

fn modifier_named(name: &str) -> Option<Modifiers> {
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

fn key_named(name: &str) -> Result<LogicalKey, Error> {
    if name.is_empty() {
        return Err(Error::fixed(ErrorKind::EmptyShortcut, "empty shortcut"));
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
        .ok_or_else(|| Error::message(ErrorKind::UnknownKey, format_args!("unknown key `{name}`")))
}

fn named_case_insensitive(name: &str) -> Option<Key> {
    let mut chars = name.chars();
    let first = chars.next()?.to_uppercase().collect::<String>();
    let candidate = first + &chars.as_str().to_ascii_lowercase();
    Key::from_str(&candidate)
        .ok()
        .filter(|key| !matches!(key, Key::Unidentified))
}
