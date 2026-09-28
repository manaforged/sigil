use smallvec::SmallVec;

use super::{color_key, scale};
use crate::style::{AlignKind, Dimension, Elevation, JustifyKind, Px, StyleAtom};

pub(super) fn parse_prefixed_token(token: &str) -> Option<StyleAtom> {
    spacing_atom(token)
        .or_else(|| dimension_atom(token))
        .or_else(|| align_atom(token))
        .or_else(|| justify_atom(token))
        .or_else(|| opacity_atom(token))
        .or_else(|| border_atom(token))
        .or_else(|| grid_atom(token))
        .or_else(|| theme_atom(token))
}

fn grid_atom(token: &str) -> Option<StyleAtom> {
    if let Some(n) = token.strip_prefix("grid-cols-") {
        return n.parse::<u16>().ok().map(StyleAtom::GridColumns);
    }
    if let Some(n) = token.strip_prefix("grid-rows-") {
        return n.parse::<u16>().ok().map(StyleAtom::GridRows);
    }
    None
}

pub(super) fn shadow_atom(token: &str) -> Option<StyleAtom> {
    let level = match token {
        "shadow-sm" => Elevation::Sm,
        "shadow-md" => Elevation::Md,
        "shadow-lg" => Elevation::Lg,
        "shadow-xl" => Elevation::Xl,
        _ => return None,
    };
    Some(StyleAtom::Shadow(level))
}

pub(super) fn push_border_side_atoms(token: &str, atoms: &mut SmallVec<[StyleAtom; 16]>) -> bool {
    let Some(rest) = token.strip_prefix("border-") else {
        return false;
    };
    let Some((sides, width)) = border_side_spec(rest) else {
        return false;
    };
    for atom in sides {
        match atom {
            't' => atoms.push(StyleAtom::BorderTop(width)),
            'r' => atoms.push(StyleAtom::BorderRight(width)),
            'b' => atoms.push(StyleAtom::BorderBottom(width)),
            'l' => atoms.push(StyleAtom::BorderLeft(width)),
            _ => {}
        }
    }
    true
}

fn border_side_spec(rest: &str) -> Option<(&'static [char], Px)> {
    let (side, width) = match rest.split_once('-') {
        Some((side, raw)) => (side, raw.parse::<i16>().ok().filter(|n| *n >= 0)?),
        None => (rest, 1),
    };
    let sides: &[char] = match side {
        "t" => &['t'],
        "r" => &['r'],
        "b" => &['b'],
        "l" => &['l'],
        "x" => &['l', 'r'],
        "y" => &['t', 'b'],
        _ => return None,
    };
    Some((sides, Px(width)))
}

fn border_atom(token: &str) -> Option<StyleAtom> {
    let value = token.strip_prefix("border-")?;
    if let Ok(width) = value.parse::<i16>() {
        return (width >= 0).then_some(StyleAtom::BorderWidth(Px(width)));
    }
    color_key(value).map(StyleAtom::BorderThemeRef)
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
    if let Some(value) = token.strip_prefix("bg-") {
        return color_key(value).map(StyleAtom::BackgroundThemeRef);
    }
    token
        .strip_prefix("text-")
        .and_then(color_key)
        .map(StyleAtom::TextThemeRef)
}
