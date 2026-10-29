use super::{color_key, scale};
use crate::style::{
    AlignKind, ColorKey, Dimension, DisplayKind, Elevation, FlexDir, FlexWrapKind, FontFamily,
    JustifyKind, Px, StyleAtom, TextDecoration,
};

#[derive(Clone, Copy)]
enum Value {
    Scale(fn(Px) -> StyleAtom),
    Length(fn(Dimension) -> StyleAtom),
    Width(fn(Px) -> StyleAtom),
    Count(fn(u16) -> StyleAtom),
    Percent(fn(u16) -> StyleAtom),
}

static KEYWORDS: phf::Map<&'static str, StyleAtom> = phf::phf_map! {
    "flex" => StyleAtom::Display(DisplayKind::Flex),
    "block" => StyleAtom::Display(DisplayKind::Block),
    "grid" => StyleAtom::Display(DisplayKind::Grid),
    "hidden" => StyleAtom::Display(DisplayKind::None),
    "flex-row" => StyleAtom::FlexDirection(FlexDir::Row),
    "flex-col" => StyleAtom::FlexDirection(FlexDir::Column),
    "flex-row-reverse" => StyleAtom::FlexDirection(FlexDir::RowReverse),
    "flex-col-reverse" => StyleAtom::FlexDirection(FlexDir::ColumnReverse),
    "flex-wrap" => StyleAtom::FlexWrap(FlexWrapKind::Wrap),
    "flex-nowrap" => StyleAtom::FlexWrap(FlexWrapKind::NoWrap),
    "w-full" => StyleAtom::Width(Dimension::Full),
    "h-full" => StyleAtom::Height(Dimension::Full),
    "w-auto" => StyleAtom::Width(Dimension::Auto),
    "h-auto" => StyleAtom::Height(Dimension::Auto),
    "grow" => StyleAtom::FlexGrow(1000),
    "grow-0" => StyleAtom::FlexGrow(0),
    "shrink" => StyleAtom::FlexShrink(1000),
    "shrink-0" => StyleAtom::FlexShrink(0),
    "rounded" => StyleAtom::BorderRadius(Px(4)),
    "rounded-sm" => StyleAtom::BorderRadius(Px(2)),
    "rounded-md" => StyleAtom::BorderRadius(Px(6)),
    "rounded-lg" => StyleAtom::BorderRadius(Px(8)),
    "rounded-full" => StyleAtom::BorderRadius(Px(9_999)),
    "border" => StyleAtom::BorderWidth(Px(1)),
    "shadow" => StyleAtom::Shadow(Elevation::Base),
    "shadow-sm" => StyleAtom::Shadow(Elevation::Sm),
    "shadow-md" => StyleAtom::Shadow(Elevation::Md),
    "shadow-lg" => StyleAtom::Shadow(Elevation::Lg),
    "shadow-xl" => StyleAtom::Shadow(Elevation::Xl),
    "text-xs" => StyleAtom::FontSize(Px(12)),
    "text-sm" => StyleAtom::FontSize(Px(14)),
    "text-base" => StyleAtom::FontSize(Px(16)),
    "text-lg" => StyleAtom::FontSize(Px(18)),
    "text-xl" => StyleAtom::FontSize(Px(20)),
    "underline" => StyleAtom::TextDecoration(TextDecoration::Underline),
    "line-through" => StyleAtom::TextDecoration(TextDecoration::LineThrough),
    "no-underline" => StyleAtom::TextDecoration(TextDecoration::None),
    "font-normal" => StyleAtom::FontWeight(400),
    "font-medium" => StyleAtom::FontWeight(500),
    "font-semibold" => StyleAtom::FontWeight(600),
    "font-bold" => StyleAtom::FontWeight(700),
    "italic" => StyleAtom::FontItalic(true),
    "not-italic" => StyleAtom::FontItalic(false),
    "font-sans" => StyleAtom::FontFamily(FontFamily::Sans.name()),
    "font-mono" => StyleAtom::FontFamily(FontFamily::Mono.name()),
    "font-serif" => StyleAtom::FontFamily(FontFamily::Serif.name()),
    "font-icon" => StyleAtom::FontFamily(FontFamily::Icon.name()),
    "font-display" => StyleAtom::FontFamily(FontFamily::Display.name()),
    "tabular-nums" => StyleAtom::TabularNums(true),
    "tracking-tighter" => StyleAtom::LetterSpacing(-50),
    "tracking-tight" => StyleAtom::LetterSpacing(-25),
    "tracking-normal" => StyleAtom::LetterSpacing(0),
    "tracking-wide" => StyleAtom::LetterSpacing(25),
    "tracking-wider" => StyleAtom::LetterSpacing(50),
    "tracking-widest" => StyleAtom::LetterSpacing(100),
    "items-start" => StyleAtom::AlignItems(AlignKind::Start),
    "items-center" => StyleAtom::AlignItems(AlignKind::Center),
    "items-end" => StyleAtom::AlignItems(AlignKind::End),
    "items-stretch" => StyleAtom::AlignItems(AlignKind::Stretch),
    "justify-start" => StyleAtom::JustifyContent(JustifyKind::Start),
    "justify-center" => StyleAtom::JustifyContent(JustifyKind::Center),
    "justify-end" => StyleAtom::JustifyContent(JustifyKind::End),
    "justify-between" => StyleAtom::JustifyContent(JustifyKind::Between),
    "justify-around" => StyleAtom::JustifyContent(JustifyKind::Around),
    "justify-evenly" => StyleAtom::JustifyContent(JustifyKind::Evenly),
};

const PREFIXES: [(&str, Value); 28] = [
    ("gap-x-", Value::Scale(StyleAtom::GapX)),
    ("gap-y-", Value::Scale(StyleAtom::GapY)),
    ("p-", Value::Scale(StyleAtom::PaddingAll)),
    ("px-", Value::Scale(StyleAtom::PaddingX)),
    ("py-", Value::Scale(StyleAtom::PaddingY)),
    ("pt-", Value::Scale(StyleAtom::PaddingTop)),
    ("pr-", Value::Scale(StyleAtom::PaddingRight)),
    ("pb-", Value::Scale(StyleAtom::PaddingBottom)),
    ("pl-", Value::Scale(StyleAtom::PaddingLeft)),
    ("m-", Value::Scale(StyleAtom::MarginAll)),
    ("mx-", Value::Scale(StyleAtom::MarginX)),
    ("my-", Value::Scale(StyleAtom::MarginY)),
    ("mt-", Value::Scale(StyleAtom::MarginTop)),
    ("mr-", Value::Scale(StyleAtom::MarginRight)),
    ("mb-", Value::Scale(StyleAtom::MarginBottom)),
    ("ml-", Value::Scale(StyleAtom::MarginLeft)),
    ("gap-", Value::Scale(StyleAtom::Gap)),
    ("leading-", Value::Scale(StyleAtom::LineHeight)),
    ("min-w-", Value::Length(StyleAtom::MinWidth)),
    ("max-w-", Value::Length(StyleAtom::MaxWidth)),
    ("min-h-", Value::Length(StyleAtom::MinHeight)),
    ("max-h-", Value::Length(StyleAtom::MaxHeight)),
    ("w-", Value::Length(StyleAtom::Width)),
    ("h-", Value::Length(StyleAtom::Height)),
    ("opacity-", Value::Percent(StyleAtom::Opacity)),
    ("border-", Value::Width(StyleAtom::BorderWidth)),
    ("grid-cols-", Value::Count(StyleAtom::GridColumns)),
    ("grid-rows-", Value::Count(StyleAtom::GridRows)),
];

type ColorPrefix = (&'static str, fn(ColorKey) -> StyleAtom);

const COLOR_PREFIXES: [ColorPrefix; 3] = [
    ("border-", StyleAtom::BorderThemeRef),
    ("bg-", StyleAtom::BackgroundThemeRef),
    ("text-", StyleAtom::TextThemeRef),
];

pub(super) fn parse_table_token(token: &str) -> Option<StyleAtom> {
    if let Some(atom) = KEYWORDS.get(token) {
        return Some(*atom);
    }
    for (prefix, value) in PREFIXES {
        let Some(raw) = token.strip_prefix(prefix) else {
            continue;
        };
        match apply(value, raw) {
            Some(Parsed::Atom(atom)) => return Some(atom),
            Some(Parsed::Refused) => return None,
            None => {}
        }
    }
    COLOR_PREFIXES
        .iter()
        .find_map(|(prefix, make)| token.strip_prefix(prefix).and_then(color_key).map(*make))
}

enum Parsed {
    Atom(StyleAtom),
    Refused,
}

fn apply(value: Value, raw: &str) -> Option<Parsed> {
    match value {
        Value::Scale(make) => scale(raw).map(|px| Parsed::Atom(make(px))),
        Value::Length(make) => scale(raw).map(|px| Parsed::Atom(make(Dimension::Px(px)))),
        Value::Width(make) => raw.parse::<i16>().ok().map(|width| {
            if width >= 0 {
                Parsed::Atom(make(Px(width)))
            } else {
                Parsed::Refused
            }
        }),
        Value::Count(make) => raw.parse::<u16>().ok().map(|n| Parsed::Atom(make(n))),
        Value::Percent(make) => raw
            .parse::<u16>()
            .ok()
            .filter(|n| *n <= 100)
            .map(|n| Parsed::Atom(make(n.saturating_mul(100)))),
    }
}
