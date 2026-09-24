use core::fmt;

use smallvec::SmallVec;

use crate::style::{
    Dimension, DisplayKind, Elevation, FlexDir, FlexWrapKind, OverflowKind, Px, StateSlot,
    StyleAtom, TextDecoration,
};

mod atoms;

use atoms::{parse_prefixed_token, push_border_side_atoms, shadow_atom};

const STRUCTURAL_TOKENS: [&str; 80] = [
    "p-4",
    "px-4",
    "py-4",
    "pt-4",
    "pr-4",
    "pb-4",
    "pl-4",
    "m-4",
    "mx-4",
    "my-4",
    "mt-4",
    "mr-4",
    "mb-4",
    "ml-4",
    "gap-4",
    "w-4",
    "h-4",
    "min-w-4",
    "max-w-4",
    "min-h-4",
    "max-h-4",
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
    "font-bold",
    "italic",
    "not-italic",
    "font-sans",
    "font-mono",
    "font-serif",
    "leading-4",
    "opacity-0",
    "opacity-100",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
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

pub fn parse_class_string(input: &str) -> Result<SmallVec<[StyleAtom; 16]>, ParseError> {
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

pub struct StatefulToken {
    pub slot: Option<StateSlot>,
    pub atoms: SmallVec<[StyleAtom; 2]>,
}

pub fn parse_stateful_token(token: &str) -> Result<StatefulToken, ParseError> {
    let Some((prefix, rest)) = token.split_once(':') else {
        return Ok(StatefulToken {
            slot: None,
            atoms: parse_class_string(token)?.into_iter().collect(),
        });
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
    Ok(StatefulToken {
        slot: Some(slot),
        atoms: parse_class_string(rest)?.into_iter().collect(),
    })
}

fn parse_token(token: &str) -> Result<StyleAtom, ParseError> {
    if malformed_theme_token(token) {
        return Err(unknown(token));
    }
    match token {
        "flex" => return Ok(StyleAtom::Display(DisplayKind::Flex)),
        "block" => return Ok(StyleAtom::Display(DisplayKind::Block)),
        "grid" => return Ok(StyleAtom::Display(DisplayKind::Grid)),
        "hidden" => return Ok(StyleAtom::Display(DisplayKind::None)),
        "flex-row" => return Ok(StyleAtom::FlexDirection(FlexDir::Row)),
        "flex-col" => return Ok(StyleAtom::FlexDirection(FlexDir::Column)),
        "flex-row-reverse" => return Ok(StyleAtom::FlexDirection(FlexDir::RowReverse)),
        "flex-col-reverse" => return Ok(StyleAtom::FlexDirection(FlexDir::ColumnReverse)),
        "flex-wrap" => return Ok(StyleAtom::FlexWrap(FlexWrapKind::Wrap)),
        "flex-nowrap" => return Ok(StyleAtom::FlexWrap(FlexWrapKind::NoWrap)),
        "w-full" => return Ok(StyleAtom::Width(Dimension::Full)),
        "h-full" => return Ok(StyleAtom::Height(Dimension::Full)),
        "w-auto" => return Ok(StyleAtom::Width(Dimension::Auto)),
        "h-auto" => return Ok(StyleAtom::Height(Dimension::Auto)),
        "rounded" => return Ok(StyleAtom::BorderRadius(Px(4))),
        "rounded-sm" => return Ok(StyleAtom::BorderRadius(Px(2))),
        "rounded-md" => return Ok(StyleAtom::BorderRadius(Px(6))),
        "rounded-lg" => return Ok(StyleAtom::BorderRadius(Px(8))),
        "rounded-full" => return Ok(StyleAtom::BorderRadius(Px(9_999))),
        "text-xs" => return Ok(StyleAtom::FontSize(Px(12))),
        "text-sm" => return Ok(StyleAtom::FontSize(Px(14))),
        "text-base" => return Ok(StyleAtom::FontSize(Px(16))),
        "text-lg" => return Ok(StyleAtom::FontSize(Px(18))),
        "text-xl" => return Ok(StyleAtom::FontSize(Px(20))),
        "grow" => return Ok(StyleAtom::FlexGrow(1000)),
        "grow-0" => return Ok(StyleAtom::FlexGrow(0)),
        "shrink" => return Ok(StyleAtom::FlexShrink(1000)),
        "shrink-0" => return Ok(StyleAtom::FlexShrink(0)),
        "underline" => return Ok(StyleAtom::TextDecoration(TextDecoration::Underline)),
        "line-through" => return Ok(StyleAtom::TextDecoration(TextDecoration::LineThrough)),
        "no-underline" => return Ok(StyleAtom::TextDecoration(TextDecoration::None)),
        "border" => return Ok(StyleAtom::BorderWidth(Px(1))),
        "shadow" => return Ok(StyleAtom::Shadow(Elevation::Base)),
        _ => {}
    }
    parse_font_token(token)
        .or_else(|| parse_tracking_token(token))
        .or_else(|| shadow_atom(token))
        .or_else(|| parse_prefixed_token(token))
        .ok_or_else(|| unknown(token))
}

fn parse_font_token(token: &str) -> Option<StyleAtom> {
    match token {
        "font-normal" => Some(StyleAtom::FontWeight(400)),
        "font-medium" => Some(StyleAtom::FontWeight(500)),
        "font-bold" => Some(StyleAtom::FontWeight(700)),
        "italic" => Some(StyleAtom::FontItalic(true)),
        "not-italic" => Some(StyleAtom::FontItalic(false)),
        "font-sans" => Some(StyleAtom::FontFamily("sans")),
        "font-mono" => Some(StyleAtom::FontFamily("mono")),
        "font-serif" => Some(StyleAtom::FontFamily("serif")),
        "font-icon" => Some(StyleAtom::FontFamily("icon")),
        "font-display" => Some(StyleAtom::FontFamily("display")),
        _ => None,
    }
}

fn parse_tracking_token(token: &str) -> Option<StyleAtom> {
    match token {
        "tabular-nums" => Some(StyleAtom::TabularNums(true)),
        "tracking-tighter" => Some(StyleAtom::LetterSpacing(-50)),
        "tracking-tight" => Some(StyleAtom::LetterSpacing(-25)),
        "tracking-normal" => Some(StyleAtom::LetterSpacing(0)),
        "tracking-wide" => Some(StyleAtom::LetterSpacing(25)),
        "tracking-wider" => Some(StyleAtom::LetterSpacing(50)),
        "tracking-widest" => Some(StyleAtom::LetterSpacing(100)),
        _ => None,
    }
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
    let unit = value.parse::<i16>().ok()?;
    unit.checked_mul(4).map(Px)
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

#[macro_export]
macro_rules! classes {
    ($($input:literal),+ $(,)?) => {
        $crate::style::parse_classes(concat!($($input, " "),+))
    };
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod parser_tests;
