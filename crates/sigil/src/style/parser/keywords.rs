use crate::style::{
    Dimension, DisplayKind, Elevation, FlexDir, FlexWrapKind, Px, StyleAtom, TextDecoration,
};

pub(super) fn parse_display_token(token: &str) -> Option<StyleAtom> {
    match token {
        "flex" => Some(StyleAtom::Display(DisplayKind::Flex)),
        "block" => Some(StyleAtom::Display(DisplayKind::Block)),
        "grid" => Some(StyleAtom::Display(DisplayKind::Grid)),
        "hidden" => Some(StyleAtom::Display(DisplayKind::None)),
        "flex-row" => Some(StyleAtom::FlexDirection(FlexDir::Row)),
        "flex-col" => Some(StyleAtom::FlexDirection(FlexDir::Column)),
        "flex-row-reverse" => Some(StyleAtom::FlexDirection(FlexDir::RowReverse)),
        "flex-col-reverse" => Some(StyleAtom::FlexDirection(FlexDir::ColumnReverse)),
        "flex-wrap" => Some(StyleAtom::FlexWrap(FlexWrapKind::Wrap)),
        "flex-nowrap" => Some(StyleAtom::FlexWrap(FlexWrapKind::NoWrap)),
        _ => None,
    }
}

pub(super) fn parse_size_token(token: &str) -> Option<StyleAtom> {
    match token {
        "w-full" => Some(StyleAtom::Width(Dimension::Full)),
        "h-full" => Some(StyleAtom::Height(Dimension::Full)),
        "w-auto" => Some(StyleAtom::Width(Dimension::Auto)),
        "h-auto" => Some(StyleAtom::Height(Dimension::Auto)),
        "grow" => Some(StyleAtom::FlexGrow(1000)),
        "grow-0" => Some(StyleAtom::FlexGrow(0)),
        "shrink" => Some(StyleAtom::FlexShrink(1000)),
        "shrink-0" => Some(StyleAtom::FlexShrink(0)),
        _ => None,
    }
}

pub(super) fn parse_shape_token(token: &str) -> Option<StyleAtom> {
    match token {
        "rounded" => Some(StyleAtom::BorderRadius(Px(4))),
        "rounded-sm" => Some(StyleAtom::BorderRadius(Px(2))),
        "rounded-md" => Some(StyleAtom::BorderRadius(Px(6))),
        "rounded-lg" => Some(StyleAtom::BorderRadius(Px(8))),
        "rounded-full" => Some(StyleAtom::BorderRadius(Px(9_999))),
        "border" => Some(StyleAtom::BorderWidth(Px(1))),
        "shadow" => Some(StyleAtom::Shadow(Elevation::Base)),
        _ => None,
    }
}

pub(super) fn parse_text_token(token: &str) -> Option<StyleAtom> {
    match token {
        "text-xs" => Some(StyleAtom::FontSize(Px(12))),
        "text-sm" => Some(StyleAtom::FontSize(Px(14))),
        "text-base" => Some(StyleAtom::FontSize(Px(16))),
        "text-lg" => Some(StyleAtom::FontSize(Px(18))),
        "text-xl" => Some(StyleAtom::FontSize(Px(20))),
        "underline" => Some(StyleAtom::TextDecoration(TextDecoration::Underline)),
        "line-through" => Some(StyleAtom::TextDecoration(TextDecoration::LineThrough)),
        "no-underline" => Some(StyleAtom::TextDecoration(TextDecoration::None)),
        _ => None,
    }
}

pub(super) fn parse_font_token(token: &str) -> Option<StyleAtom> {
    match token {
        "font-normal" => Some(StyleAtom::FontWeight(400)),
        "font-medium" => Some(StyleAtom::FontWeight(500)),
        "font-semibold" => Some(StyleAtom::FontWeight(600)),
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

pub(super) fn parse_tracking_token(token: &str) -> Option<StyleAtom> {
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
