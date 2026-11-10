use super::color_key;
use crate::style::StyleAtom;
use crate::style::atom::{Amount, FilterAtom, FilterPart, ShadowSpec, TextShadows};

const BLURS: [(&str, u16); 8] = [
    ("none", 0),
    ("xs", 4),
    ("sm", 8),
    ("md", 12),
    ("lg", 16),
    ("xl", 24),
    ("2xl", 40),
    ("3xl", 64),
];

const AMOUNTS: [(&str, Amount, Option<u16>); 6] = [
    ("brightness", Amount::Brightness, None),
    ("contrast", Amount::Contrast, None),
    ("saturate", Amount::Saturate, None),
    ("grayscale", Amount::Grayscale, Some(100)),
    ("invert", Amount::Invert, Some(100)),
    ("sepia", Amount::Sepia, Some(100)),
];

const DROPS: [(&str, ShadowSpec); 6] = [
    ("xs", ShadowSpec::black(1, 16, 13)),
    ("sm", ShadowSpec::black(1, 32, 38)),
    ("md", ShadowSpec::black(3, 48, 31)),
    ("lg", ShadowSpec::black(4, 64, 38)),
    ("xl", ShadowSpec::black(9, 112, 26)),
    ("2xl", ShadowSpec::black(25, 400, 38)),
];

static TEXT_2XS: [ShadowSpec; 1] = [ShadowSpec::black(1, 0, 38)];
static TEXT_XS: [ShadowSpec; 1] = [ShadowSpec::black(1, 8, 51)];
static TEXT_SM: [ShadowSpec; 3] = [
    ShadowSpec::black(1, 0, 19),
    ShadowSpec::black(1, 8, 19),
    ShadowSpec::black(2, 16, 19),
];
static TEXT_MD: [ShadowSpec; 3] = [
    ShadowSpec::black(1, 8, 26),
    ShadowSpec::black(1, 16, 26),
    ShadowSpec::black(2, 32, 26),
];
static TEXT_LG: [ShadowSpec; 3] = [
    ShadowSpec::black(1, 16, 26),
    ShadowSpec::black(3, 16, 26),
    ShadowSpec::black(4, 64, 26),
];

pub(super) fn parse_filter_token(token: &str) -> Option<StyleAtom> {
    let part = |part: FilterPart| Some(StyleAtom::Filter(FilterAtom(part)));
    if let Some(degrees) = token.strip_prefix("-hue-rotate-") {
        return part(FilterPart::HueRotate(-degrees.parse::<i16>().ok()?));
    }
    if let Some(degrees) = token.strip_prefix("hue-rotate-") {
        return part(FilterPart::HueRotate(degrees.parse::<i16>().ok()?));
    }
    if let Some(size) = token.strip_prefix("blur-") {
        let (_, px) = BLURS.iter().find(|(name, _)| *name == size)?;
        return part(FilterPart::Blur(px * 16));
    }
    if let Some(rest) = token.strip_prefix("drop-shadow-") {
        return drop_shadow(rest);
    }
    if let Some(rest) = token.strip_prefix("text-shadow-") {
        return text_shadow(rest);
    }
    amount(token).and_then(part)
}

fn amount(token: &str) -> Option<FilterPart> {
    AMOUNTS.iter().find_map(|(name, kind, bare)| {
        let rest = token.strip_prefix(name)?;
        let percent = match rest.strip_prefix('-') {
            Some(number) => number.parse::<u16>().ok()?,
            None if rest.is_empty() => (*bare)?,
            None => return None,
        };
        Some(FilterPart::Amount(*kind, percent))
    })
}

fn drop_shadow(rest: &str) -> Option<StyleAtom> {
    if rest == "none" {
        return Some(StyleAtom::Filter(FilterAtom(FilterPart::DropShadow(None))));
    }
    if let Some((_, spec)) = DROPS.iter().find(|(name, _)| *name == rest) {
        return Some(StyleAtom::Filter(FilterAtom(FilterPart::DropShadow(Some(
            *spec,
        )))));
    }
    color_key(rest).map(StyleAtom::DropShadowColor)
}

fn text_shadow(rest: &str) -> Option<StyleAtom> {
    let list = match rest {
        "none" => TextShadows::None,
        "2xs" => TextShadows::Preset(&TEXT_2XS),
        "xs" => TextShadows::Preset(&TEXT_XS),
        "sm" => TextShadows::Preset(&TEXT_SM),
        "md" => TextShadows::Preset(&TEXT_MD),
        "lg" => TextShadows::Preset(&TEXT_LG),
        _ => return color_key(rest).map(StyleAtom::TextShadowColor),
    };
    Some(StyleAtom::Filter(FilterAtom(FilterPart::TextShadow(list))))
}
