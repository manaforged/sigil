//! Runtime utility-class parser.

use core::fmt;

use smallvec::SmallVec;

use crate::{
    AlignKind, Dimension, DisplayKind, Elevation, FlexDir, FlexWrapKind, JustifyKind, OverflowKind,
    Px, StateSlot, StyleAtom,
};

const STRUCTURAL_TOKENS: [&str; 77] = [
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
    "text-on-surface",
    "border",
    "border-2",
    "border-accent",
    "font-normal",
    "font-medium",
    "font-bold",
    "font-sans",
    "font-mono",
    "font-serif",
    "leading-4",
    "opacity-0",
    "opacity-100",
];

/// Runtime parser error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Token was not recognised by the structural grammar.
    Unknown {
        /// The original token.
        token: String,
        /// Three nearest known structural tokens.
        suggestions: [String; 3],
    },
    /// Responsive prefixes are not implemented in this phase.
    ResponsiveNotSupported {
        /// Prefix that was rejected.
        prefix: String,
    },
    /// Pseudo-state prefixes are not implemented in this phase.
    PseudoNotSupported {
        /// Prefix that was rejected.
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

/// Parse a utility-class string into style atoms.
///
/// # Errors
///
/// Returns a [`ParseError`] when the string contains unsupported prefixes
/// or an unknown structural utility.
pub fn parse_class_string(input: &str) -> Result<SmallVec<[StyleAtom; 16]>, ParseError> {
    let mut atoms = SmallVec::new();
    for token in input.split(' ').filter(|token| !token.is_empty()) {
        reject_prefix(token)?;
        if push_overflow_atoms(token, &mut atoms)? {
            continue;
        }
        atoms.push(parse_token(token)?);
    }
    Ok(atoms)
}

/// A parsed class token: the overlay slot it targets (`None` for an
/// unprefixed base token) and the atoms it expands to.
pub struct StatefulToken {
    /// Overlay slot the atoms belong to, or `None` for base atoms.
    pub slot: Option<StateSlot>,
    /// Atoms the token body expands to.
    pub atoms: SmallVec<[StyleAtom; 2]>,
}

/// Parse a single class token that may carry a state prefix.
///
/// `hover:` / `focus:` / `active:` (or `pressed:`) / `disabled:` route into
/// their [`StateSlot`]; an unprefixed token yields base atoms; any other
/// prefix errors exactly as [`parse_class_string`] would.
///
/// This is the stateful counterpart of [`parse_class_string`]: it is what
/// lets `interactive(...).class("bg-surface hover:bg-accent-9")` express a
/// hover overlay through a class string instead of a `.hover(|s| …)`
/// closure. A plain `container().class(...)` still rejects state prefixes,
/// because a bare container has no interaction channel to drive them.
///
/// # Errors
///
/// Returns a [`ParseError`] for a responsive prefix, an unknown pseudo
/// prefix, or an unknown utility in the token body.
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
        "font-normal" => return Ok(StyleAtom::FontWeight(400)),
        "font-medium" => return Ok(StyleAtom::FontWeight(500)),
        "font-bold" => return Ok(StyleAtom::FontWeight(700)),
        "font-sans" => return Ok(StyleAtom::FontFamily("sans")),
        "font-mono" => return Ok(StyleAtom::FontFamily("mono")),
        "font-serif" => return Ok(StyleAtom::FontFamily("serif")),
        "font-icon" => return Ok(StyleAtom::FontFamily("icon")),
        "grow" => return Ok(StyleAtom::FlexGrow(1000)),
        "grow-0" => return Ok(StyleAtom::FlexGrow(0)),
        "shrink" => return Ok(StyleAtom::FlexShrink(1000)),
        "shrink-0" => return Ok(StyleAtom::FlexShrink(0)),
        "border" => return Ok(StyleAtom::BorderWidth(Px(1))),
        "shadow" => return Ok(StyleAtom::Shadow(Elevation::Base)),
        _ => {}
    }
    if let Some(atom) = shadow_atom(token) {
        return Ok(atom);
    }
    parse_prefixed_token(token).ok_or_else(|| unknown(token))
}

/// `overflow-*` expands to both axes, `overflow-x-*`/`overflow-y-*` to
/// one; returns `Ok(true)` when the token was consumed.
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

fn parse_prefixed_token(token: &str) -> Option<StyleAtom> {
    spacing_atom(token)
        .or_else(|| dimension_atom(token))
        .or_else(|| align_atom(token))
        .or_else(|| justify_atom(token))
        .or_else(|| opacity_atom(token))
        .or_else(|| border_atom(token))
        .or_else(|| grid_atom(token))
        .or_else(|| theme_atom(token))
}

/// `grid-cols-N` / `grid-rows-N` map to N equal grid tracks (and imply
/// `display: grid` at the layout adapter).
fn grid_atom(token: &str) -> Option<StyleAtom> {
    if let Some(n) = token.strip_prefix("grid-cols-") {
        return n.parse::<u16>().ok().map(StyleAtom::GridColumns);
    }
    if let Some(n) = token.strip_prefix("grid-rows-") {
        return n.parse::<u16>().ok().map(StyleAtom::GridRows);
    }
    None
}

/// `shadow-<level>` maps to a drop-shadow elevation preset. Bare
/// `shadow` (the Tailwind default) is handled in `parse_token`.
fn shadow_atom(token: &str) -> Option<StyleAtom> {
    let level = match token {
        "shadow-sm" => Elevation::Sm,
        "shadow-md" => Elevation::Md,
        "shadow-lg" => Elevation::Lg,
        "shadow-xl" => Elevation::Xl,
        _ => return None,
    };
    Some(StyleAtom::Shadow(level))
}

/// `border-N` is a literal pixel width (Tailwind shape: `border-2` is
/// 2px, not on the 4px spacing scale); `border-<token>` is a theme
/// colour reference.
fn border_atom(token: &str) -> Option<StyleAtom> {
    let value = token.strip_prefix("border-")?;
    if let Ok(width) = value.parse::<i16>() {
        return (width >= 0).then_some(StyleAtom::BorderWidth(Px(width)));
    }
    Some(StyleAtom::BorderThemeRef(leak_token(value)))
}

fn spacing_atom(token: &str) -> Option<StyleAtom> {
    if let Some(value) = token.strip_prefix("gap-x-") {
        return Some(StyleAtom::GapX(scale(value)?));
    }
    if let Some(value) = token.strip_prefix("gap-y-") {
        return Some(StyleAtom::GapY(scale(value)?));
    }
    let (prefix, value) = token.split_once('-')?;
    let px = scale(value)?;
    match prefix {
        "p" => Some(StyleAtom::PaddingAll(px)),
        "px" => Some(StyleAtom::PaddingX(px)),
        "py" => Some(StyleAtom::PaddingY(px)),
        "pt" => Some(StyleAtom::PaddingTop(px)),
        "pr" => Some(StyleAtom::PaddingRight(px)),
        "pb" => Some(StyleAtom::PaddingBottom(px)),
        "pl" => Some(StyleAtom::PaddingLeft(px)),
        "m" => Some(StyleAtom::MarginAll(px)),
        "mx" => Some(StyleAtom::MarginX(px)),
        "my" => Some(StyleAtom::MarginY(px)),
        "mt" => Some(StyleAtom::MarginTop(px)),
        "mr" => Some(StyleAtom::MarginRight(px)),
        "mb" => Some(StyleAtom::MarginBottom(px)),
        "ml" => Some(StyleAtom::MarginLeft(px)),
        "gap" => Some(StyleAtom::Gap(px)),
        "leading" => Some(StyleAtom::LineHeight(px)),
        _ => None,
    }
}

fn dimension_atom(token: &str) -> Option<StyleAtom> {
    if let Some(value) = token.strip_prefix("min-w-") {
        return Some(StyleAtom::MinWidth(Dimension::Px(scale(value)?)));
    }
    if let Some(value) = token.strip_prefix("max-w-") {
        return Some(StyleAtom::MaxWidth(Dimension::Px(scale(value)?)));
    }
    if let Some(value) = token.strip_prefix("min-h-") {
        return Some(StyleAtom::MinHeight(Dimension::Px(scale(value)?)));
    }
    if let Some(value) = token.strip_prefix("max-h-") {
        return Some(StyleAtom::MaxHeight(Dimension::Px(scale(value)?)));
    }
    let (prefix, value) = token.split_once('-')?;
    let dimension = Dimension::Px(scale(value)?);
    match prefix {
        "w" => Some(StyleAtom::Width(dimension)),
        "h" => Some(StyleAtom::Height(dimension)),
        _ => None,
    }
}

fn align_atom(token: &str) -> Option<StyleAtom> {
    let value = token.strip_prefix("items-")?;
    match value {
        "start" => Some(StyleAtom::AlignItems(AlignKind::Start)),
        "center" => Some(StyleAtom::AlignItems(AlignKind::Center)),
        "end" => Some(StyleAtom::AlignItems(AlignKind::End)),
        "stretch" => Some(StyleAtom::AlignItems(AlignKind::Stretch)),
        _ => None,
    }
}

fn justify_atom(token: &str) -> Option<StyleAtom> {
    let value = token.strip_prefix("justify-")?;
    match value {
        "start" => Some(StyleAtom::JustifyContent(JustifyKind::Start)),
        "center" => Some(StyleAtom::JustifyContent(JustifyKind::Center)),
        "end" => Some(StyleAtom::JustifyContent(JustifyKind::End)),
        "between" => Some(StyleAtom::JustifyContent(JustifyKind::Between)),
        "around" => Some(StyleAtom::JustifyContent(JustifyKind::Around)),
        "evenly" => Some(StyleAtom::JustifyContent(JustifyKind::Evenly)),
        _ => None,
    }
}

fn opacity_atom(token: &str) -> Option<StyleAtom> {
    let value = token.strip_prefix("opacity-")?.parse::<u16>().ok()?;
    (value <= 100).then_some(StyleAtom::Opacity(value.saturating_mul(100)))
}

fn theme_atom(token: &str) -> Option<StyleAtom> {
    token
        .strip_prefix("bg-")
        .map(|value| StyleAtom::BackgroundThemeRef(leak_token(value)))
        .or_else(|| {
            token
                .strip_prefix("text-")
                .map(|value| StyleAtom::TextThemeRef(leak_token(value)))
        })
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

fn leak_token(token: &str) -> &'static str {
    Box::leak(token.to_owned().into_boxed_str())
}
