use core::fmt;

use smallvec::SmallVec;

#[cfg(any(test, fuzzing))]
use crate::style::StateSlot;
use crate::style::{OverflowKind, Px, StyleAtom};

mod atoms;
mod keywords;

use atoms::{parse_prefixed_token, push_border_side_atoms, shadow_atom};
use keywords::{
    parse_display_token, parse_font_token, parse_shape_token, parse_size_token, parse_text_token,
    parse_tracking_token,
};

const STRUCTURAL_TOKENS: [&str; 81] = [
    "p-16",
    "px-16",
    "py-16",
    "pt-16",
    "pr-16",
    "pb-16",
    "pl-16",
    "m-16",
    "mx-16",
    "my-16",
    "mt-16",
    "mr-16",
    "mb-16",
    "ml-16",
    "gap-16",
    "w-16",
    "h-16",
    "min-w-16",
    "max-w-16",
    "min-h-16",
    "max-h-16",
    "w-full",
    "h-full",
    "w-auto",
    "h-auto",
    "flex",
    "block",
    "hidden",
    "flex-row",
    "flex-col",
    "flex-row-reverse",
    "flex-col-reverse",
    "items-start",
    "items-center",
    "items-end",
    "items-stretch",
    "justify-start",
    "justify-center",
    "justify-end",
    "justify-between",
    "justify-around",
    "grow",
    "grow-0",
    "shrink",
    "shrink-0",
    "overflow-hidden",
    "overflow-scroll",
    "overflow-visible",
    "rounded",
    "rounded-sm",
    "rounded-md",
    "rounded-lg",
    "rounded-full",
    "bg-accent",
    "bg-border",
    "bg-surface",
    "bg-surface-hover",
    "bg-surface-pressed",
    "bg-surface-pressed-strong",
    "text-xs",
    "text-sm",
    "text-base",
    "text-lg",
    "text-xl",
    "tabular-nums",
    "text-on-surface",
    "border",
    "border-2",
    "border-accent",
    "font-normal",
    "font-medium",
    "font-semibold",
    "font-bold",
    "italic",
    "not-italic",
    "font-sans",
    "font-mono",
    "font-serif",
    "leading-16",
    "opacity-0",
    "opacity-100",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ParseError {
    Unknown {
        token: String,
        suggestions: [String; 3],
    },
    ResponsiveNotSupported {
        prefix: String,
    },
    PseudoNotSupported {
        prefix: String,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown { token, suggestions } => write!(
                f,
                "Unknown structural token '{token}'; nearest: '{}', '{}', '{}'",
                suggestions[0], suggestions[1], suggestions[2],
            ),
            Self::ResponsiveNotSupported { prefix } => {
                write!(f, "Responsive prefix '{prefix}:' is not supported")
            }
            Self::PseudoNotSupported { prefix } => {
                write!(f, "Pseudo prefix '{prefix}:' is not supported")
            }
        }
    }
}

impl std::error::Error for ParseError {}

pub(crate) fn parse_class_string(input: &str) -> Result<SmallVec<[StyleAtom; 16]>, ParseError> {
    let mut atoms = SmallVec::new();
    for token in input.split(' ').filter(|token| !token.is_empty()) {
        reject_prefix(token)?;
        if push_overflow_atoms(token, &mut atoms)? {
            continue;
        }
        if push_border_side_atoms(token, &mut atoms) {
            continue;
        }
        atoms.push(parse_token(token)?);
    }
    Ok(atoms)
}

#[cfg(any(test, fuzzing))]
pub(crate) type StatefulAtoms = (Option<StateSlot>, SmallVec<[StyleAtom; 16]>);

#[cfg(any(test, fuzzing))]
pub(crate) fn parse_stateful_token(token: &str) -> Result<StatefulAtoms, ParseError> {
    let Some((prefix, rest)) = token.split_once(':') else {
        return Ok((None, parse_class_string(token)?));
    };
    let slot = match prefix {
        "hover" => StateSlot::Hovered,
        "focus" => StateSlot::Focused,
        "active" | "pressed" => StateSlot::Pressed,
        "disabled" => StateSlot::Disabled,
        "sm" | "md" | "lg" => {
            return Err(ParseError::ResponsiveNotSupported {
                prefix: prefix.to_owned(),
            });
        }
        _ => {
            return Err(ParseError::PseudoNotSupported {
                prefix: prefix.to_owned(),
            });
        }
    };
    Ok((Some(slot), parse_class_string(rest)?))
}

fn parse_token(token: &str) -> Result<StyleAtom, ParseError> {
    if malformed_theme_token(token) {
        return Err(unknown(token));
    }
    parse_display_token(token)
        .or_else(|| parse_size_token(token))
        .or_else(|| parse_shape_token(token))
        .or_else(|| parse_text_token(token))
        .or_else(|| parse_font_token(token))
        .or_else(|| parse_tracking_token(token))
        .or_else(|| shadow_atom(token))
        .or_else(|| parse_prefixed_token(token))
        .ok_or_else(|| unknown(token))
}

fn push_overflow_atoms(
    token: &str,
    atoms: &mut SmallVec<[StyleAtom; 16]>,
) -> Result<bool, ParseError> {
    let Some(rest) = token.strip_prefix("overflow-") else {
        return Ok(false);
    };
    if let Some(value) = rest.strip_prefix("x-") {
        let kind = overflow_kind(value).ok_or_else(|| unknown(token))?;
        atoms.push(StyleAtom::OverflowX(kind));
        return Ok(true);
    }
    if let Some(value) = rest.strip_prefix("y-") {
        let kind = overflow_kind(value).ok_or_else(|| unknown(token))?;
        atoms.push(StyleAtom::OverflowY(kind));
        return Ok(true);
    }
    let kind = overflow_kind(rest).ok_or_else(|| unknown(token))?;
    atoms.push(StyleAtom::OverflowX(kind));
    atoms.push(StyleAtom::OverflowY(kind));
    Ok(true)
}

const fn overflow_kind(value: &str) -> Option<OverflowKind> {
    match value.as_bytes() {
        b"visible" => Some(OverflowKind::Visible),
        b"hidden" => Some(OverflowKind::Hidden),
        b"scroll" => Some(OverflowKind::Scroll),
        b"auto" => Some(OverflowKind::Auto),
        _ => None,
    }
}

fn reject_prefix(token: &str) -> Result<(), ParseError> {
    if let Some((prefix, _rest)) = token.split_once(':') {
        return match prefix {
            "sm" | "md" | "lg" => Err(ParseError::ResponsiveNotSupported {
                prefix: prefix.to_owned(),
            }),
            "hover" | "focus" | "active" => Err(ParseError::PseudoNotSupported {
                prefix: prefix.to_owned(),
            }),
            _ => Err(unknown(token)),
        };
    }
    Ok(())
}

fn malformed_theme_token(token: &str) -> bool {
    token.contains("${") || token.contains('}') || token.ends_with('-')
}

fn scale(value: &str) -> Option<Px> {
    value.parse::<i16>().ok().map(Px)
}

fn unknown(token: &str) -> ParseError {
    ParseError::Unknown {
        token: token.to_owned(),
        suggestions: nearest(token),
    }
}

fn nearest(token: &str) -> [String; 3] {
    let mut scored = STRUCTURAL_TOKENS
        .iter()
        .copied()
        .map(|candidate| (strsim::levenshtein(token, candidate), candidate))
        .collect::<Vec<_>>();
    scored.sort_by_key(|(score, candidate)| (*score, *candidate));
    [
        scored
            .first()
            .map_or("", |(_, candidate)| *candidate)
            .to_owned(),
        scored
            .get(1)
            .map_or("", |(_, candidate)| *candidate)
            .to_owned(),
        scored
            .get(2)
            .map_or("", |(_, candidate)| *candidate)
            .to_owned(),
    ]
}

const MAX_INTERNED_TOKENS: usize = 4096;

static INTERNED_TOKENS: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashSet<&'static str>>,
> = std::sync::LazyLock::new(std::sync::Mutex::default);

fn color_key(name: &str) -> Option<crate::style::ColorKey> {
    if let Some(token) = crate::style::Token::from_name(name) {
        return Some(crate::style::ColorKey::Token(token));
    }
    intern_token(name).map(crate::style::ColorKey::Custom)
}

fn intern_token(token: &str) -> Option<&'static str> {
    let mut tokens = INTERNED_TOKENS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(interned) = tokens.get(token) {
        return Some(*interned);
    }
    if tokens.len() >= MAX_INTERNED_TOKENS {
        return None;
    }
    let interned: &'static str = Box::leak(token.to_owned().into_boxed_str());
    tokens.insert(interned);
    drop(tokens);
    Some(interned)
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod parser_tests;
