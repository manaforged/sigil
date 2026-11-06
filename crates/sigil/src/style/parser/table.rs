use super::{color_key, scale};
use crate::layout::{AlignItems, Display, FlexDirection, FlexWrap, JustifyContent};
use crate::style::atom::{
    height_px, max_height_px, max_width_px, min_height_px, min_width_px, width_px,
};
use crate::style::atom::{BorderStyle, GradientShape, PaintAtom};
use crate::style::{
    ColorAtom, Conic, Elevation, FontFamily, GradientAxis, Radial, StyleAtom, TextDecoration,
};

#[derive(Clone, Copy)]
enum Value {
    Scale(fn(i16) -> StyleAtom),
    Length(fn(i16) -> StyleAtom),
    Width(fn(i16) -> StyleAtom),
    Count(fn(u16) -> StyleAtom),
    Percent(fn(u16) -> StyleAtom),
}

static KEYWORDS: phf::Map<&'static str, StyleAtom> = phf::phf_map! {
    "flex" => StyleAtom::Display(Display::Flex),
    "block" => StyleAtom::Display(Display::Block),
    "grid" => StyleAtom::Display(Display::Grid),
    "hidden" => StyleAtom::Display(Display::None),
    "flex-row" => StyleAtom::FlexDirection(FlexDirection::Row),
    "flex-col" => StyleAtom::FlexDirection(FlexDirection::Column),
    "flex-row-reverse" => StyleAtom::FlexDirection(FlexDirection::RowReverse),
    "flex-col-reverse" => StyleAtom::FlexDirection(FlexDirection::ColumnReverse),
    "flex-wrap" => StyleAtom::FlexWrap(FlexWrap::Wrap),
    "flex-nowrap" => StyleAtom::FlexWrap(FlexWrap::NoWrap),
    "w-full" => StyleAtom::Width(crate::style::atom::ATOM_FULL, 0),
    "h-full" => StyleAtom::Height(crate::style::atom::ATOM_FULL, 0),
    "w-auto" => StyleAtom::Width(crate::style::atom::ATOM_AUTO, 0),
    "h-auto" => StyleAtom::Height(crate::style::atom::ATOM_AUTO, 0),
    "grow" => StyleAtom::FlexGrow(1000),
    "grow-0" => StyleAtom::FlexGrow(0),
    "shrink" => StyleAtom::FlexShrink(1000),
    "shrink-0" => StyleAtom::FlexShrink(0),
    "rounded" => StyleAtom::BorderRadius(4),
    "rounded-sm" => StyleAtom::BorderRadius(2),
    "rounded-md" => StyleAtom::BorderRadius(6),
    "rounded-lg" => StyleAtom::BorderRadius(8),
    "rounded-full" => StyleAtom::BorderRadius(9_999),
    "border" => StyleAtom::BorderWidth(1),
    "shadow" => StyleAtom::Shadow(Elevation::Base),
    "shadow-sm" => StyleAtom::Shadow(Elevation::Sm),
    "shadow-md" => StyleAtom::Shadow(Elevation::Md),
    "shadow-lg" => StyleAtom::Shadow(Elevation::Lg),
    "shadow-xl" => StyleAtom::Shadow(Elevation::Xl),
    "text-xs" => StyleAtom::FontSize(12),
    "text-sm" => StyleAtom::FontSize(14),
    "text-base" => StyleAtom::FontSize(16),
    "text-lg" => StyleAtom::FontSize(18),
    "text-xl" => StyleAtom::FontSize(20),
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
    "items-start" => StyleAtom::AlignItems(AlignItems::Start),
    "items-center" => StyleAtom::AlignItems(AlignItems::Center),
    "items-end" => StyleAtom::AlignItems(AlignItems::End),
    "items-stretch" => StyleAtom::AlignItems(AlignItems::Stretch),
    "justify-start" => StyleAtom::JustifyContent(JustifyContent::Start),
    "justify-center" => StyleAtom::JustifyContent(JustifyContent::Center),
    "justify-end" => StyleAtom::JustifyContent(JustifyContent::End),
    "justify-between" => StyleAtom::JustifyContent(JustifyContent::SpaceBetween),
    "justify-around" => StyleAtom::JustifyContent(JustifyContent::SpaceAround),
    "justify-evenly" => StyleAtom::JustifyContent(JustifyContent::SpaceEvenly),
    "bg-linear-to-t" => paint(PaintAtom::Gradient(GradientShape::Linear(GradientAxis::BottomToTop))),
    "bg-linear-to-b" => paint(PaintAtom::Gradient(GradientShape::Linear(GradientAxis::TopToBottom))),
    "bg-linear-to-l" => paint(PaintAtom::Gradient(GradientShape::Linear(GradientAxis::RightToLeft))),
    "bg-linear-to-r" => paint(PaintAtom::Gradient(GradientShape::Linear(GradientAxis::LeftToRight))),
    "bg-radial" => paint(PaintAtom::Gradient(GradientShape::Radial(Radial::new()))),
    "bg-conic" => paint(PaintAtom::Gradient(GradientShape::Conic(Conic::new(0)))),
    "bg-clip-text" => paint(PaintAtom::ClipText),
    "border-solid" => paint(PaintAtom::Border(BorderStyle::Solid)),
    "border-dashed" => paint(PaintAtom::Border(BorderStyle::Dashed)),
    "border-dotted" => paint(PaintAtom::Border(BorderStyle::Dotted)),
};

const fn paint(atom: PaintAtom) -> StyleAtom {
    StyleAtom::Paint(atom)
}

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
    ("min-w-", Value::Length(min_width_px)),
    ("max-w-", Value::Length(max_width_px)),
    ("min-h-", Value::Length(min_height_px)),
    ("max-h-", Value::Length(max_height_px)),
    ("w-", Value::Length(width_px)),
    ("h-", Value::Length(height_px)),
    ("opacity-", Value::Percent(StyleAtom::Opacity)),
    ("border-", Value::Width(StyleAtom::BorderWidth)),
    ("grid-cols-", Value::Count(StyleAtom::GridColumns)),
    ("grid-rows-", Value::Count(StyleAtom::GridRows)),
];

type ColorPrefix = (&'static str, fn(ColorAtom) -> StyleAtom);

const COLOR_PREFIXES: [ColorPrefix; 3] = [
    ("border-", StyleAtom::BorderColor),
    ("bg-", StyleAtom::BackgroundColor),
    ("text-", StyleAtom::TextColor),
];

pub(super) fn parse_table_token(token: &str) -> Option<StyleAtom> {
    if let Some(atom) = KEYWORDS.get(token) {
        return Some(crate::style::text_scale::scaled(*atom));
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
    if let Some(atom) = super::paint::parse_paint_token(token) {
        return Some(atom);
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
        Value::Scale(make) => scale(raw).map(|px| Parsed::Atom(make(px.0))),
        Value::Length(make) => scale(raw).map(|px| Parsed::Atom(make(px.0))),
        Value::Width(make) => raw.parse::<i16>().ok().map(|width| {
            if width >= 0 {
                Parsed::Atom(make(width))
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
