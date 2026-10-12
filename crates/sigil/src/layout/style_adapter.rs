#![allow(clippy::redundant_pub_crate)]
#![cfg_attr(not(any(test, feature = "kittest")), allow(unreachable_pub))]

use crate::style::atom::FACTOR_SCALE;
use crate::style::{
    AlignKind, Dimension as StyleDimension, DisplayKind, FlexDir, FlexWrapKind, JustifyKind,
    OverflowKind, Px, StyleAtom,
};

use crate::layout::{
    LayoutStyle,
    style_data::{
        AlignItems, Dimension, Display, FlexDirection, FlexWrap, JustifyContent, LengthPercentage,
        LengthPercentageAuto, Overflow, Rect, Size, TrackSize,
    },
};

pub fn apply_style_atoms(atoms: &[StyleAtom], style: &mut LayoutStyle) {
    for atom in atoms {
        apply_style_atom(*atom, style);
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "exhaustive match on StyleAtom — the long no-op arm chain is intentional; a wildcard would silently swallow new atom kinds"
)]
fn apply_style_atom(atom: StyleAtom, layout: &mut LayoutStyle) {
    let s = layout.data_mut();
    match atom {
        StyleAtom::PaddingAll(px) => s.padding = Rect::splat(length(px)),
        StyleAtom::PaddingX(px) => (s.padding.left, s.padding.right) = (length(px), length(px)),
        StyleAtom::PaddingY(px) => (s.padding.top, s.padding.bottom) = (length(px), length(px)),
        StyleAtom::PaddingTop(px) => s.padding.top = length(px),
        StyleAtom::PaddingRight(px) => s.padding.right = length(px),
        StyleAtom::PaddingBottom(px) => s.padding.bottom = length(px),
        StyleAtom::PaddingLeft(px) => s.padding.left = length(px),
        StyleAtom::MarginAll(px) => s.margin = Rect::splat(length_auto(px)),
        StyleAtom::MarginX(px) => {
            (s.margin.left, s.margin.right) = (length_auto(px), length_auto(px));
        }
        StyleAtom::MarginY(px) => {
            (s.margin.top, s.margin.bottom) = (length_auto(px), length_auto(px));
        }
        StyleAtom::MarginTop(px) => s.margin.top = length_auto(px),
        StyleAtom::MarginRight(px) => s.margin.right = length_auto(px),
        StyleAtom::MarginBottom(px) => s.margin.bottom = length_auto(px),
        StyleAtom::MarginLeft(px) => s.margin.left = length_auto(px),
        StyleAtom::Gap(gap) => s.gap = Size::splat(length(gap)),
        StyleAtom::GapX(gap) => s.gap.width = length(gap),
        StyleAtom::GapY(gap) => s.gap.height = length(gap),
        StyleAtom::Width(width) => s.size.width = dimension(width),
        StyleAtom::Height(height) => s.size.height = dimension(height),
        StyleAtom::MinWidth(width) => s.min_size.width = dimension(width),
        StyleAtom::MaxWidth(width) => s.max_size.width = dimension(width),
        StyleAtom::MinHeight(height) => s.min_size.height = dimension(height),
        StyleAtom::MaxHeight(height) => s.max_size.height = dimension(height),
        StyleAtom::Display(display) => s.display = display_kind(display),
        StyleAtom::GridColumns(n) => {
            s.display = Display::Grid;
            s.grid_template_columns = vec![TrackSize::Fr(1.0); n as usize];
        }
        StyleAtom::GridRows(n) => {
            s.display = Display::Grid;
            s.grid_template_rows = vec![TrackSize::Auto; n as usize];
        }
        StyleAtom::FlexDirection(direction) => s.flex_direction = flex_direction(direction),
        StyleAtom::FlexWrap(FlexWrapKind::NoWrap) => s.flex_wrap = FlexWrap::NoWrap,
        StyleAtom::FlexWrap(FlexWrapKind::Wrap) => s.flex_wrap = FlexWrap::Wrap,
        StyleAtom::AlignItems(align) => s.align_items = Some(align_items(align)),
        StyleAtom::JustifyContent(justify) => s.justify_content = Some(justify_content(justify)),
        StyleAtom::FlexGrow(grow) => s.flex_grow = f32::from(grow) / FACTOR_SCALE,
        StyleAtom::FlexShrink(shrink) => s.flex_shrink = f32::from(shrink) / FACTOR_SCALE,
        StyleAtom::OverflowX(kind) => s.overflow.width = overflow(kind),
        StyleAtom::OverflowY(kind) => s.overflow.height = overflow(kind),
        StyleAtom::BorderWidth(width) => s.border = Rect::splat(length(width)),
        StyleAtom::BorderTop(width) => s.border.top = length(width),
        StyleAtom::BorderRight(width) => s.border.right = length(width),
        StyleAtom::BorderBottom(width) => s.border.bottom = length(width),
        StyleAtom::BorderLeft(width) => s.border.left = length(width),
        StyleAtom::BorderColor(_)
        | StyleAtom::BorderThemeRef(_)
        | StyleAtom::BorderToneStep(_)
        | StyleAtom::BackgroundColor(_)
        | StyleAtom::BackgroundThemeRef(_)
        | StyleAtom::BackgroundGradient(_)
        | StyleAtom::TextColor(_)
        | StyleAtom::TextThemeRef(_)
        | StyleAtom::BorderRadius(_)
        | StyleAtom::Radius(_)
        | StyleAtom::Shadow(_)
        | StyleAtom::Transform(_)
        | StyleAtom::Opacity(_)
        | StyleAtom::FontSize(_)
        | StyleAtom::FontWeight(_)
        | StyleAtom::FontItalic(_)
        | StyleAtom::FontFamily(_)
        | StyleAtom::LineHeight(_)
        | StyleAtom::LetterSpacing(_)
        | StyleAtom::TabularNums(_)
        | StyleAtom::TextDecoration(_)
        | StyleAtom::Transition(_)
        | StyleAtom::TransitionDuration(_)
        | StyleAtom::UnknownThemeRef(_)
        | StyleAtom::BgToneStep(_)
        | StyleAtom::FgToneStep(_)
        | StyleAtom::FgOnSolid => {}
    }
}

const fn overflow(kind: OverflowKind) -> Overflow {
    match kind {
        OverflowKind::Visible => Overflow::Visible,
        OverflowKind::Hidden => Overflow::Hidden,
        OverflowKind::Scroll | OverflowKind::Auto => Overflow::Scroll,
    }
}

fn length(px: Px) -> LengthPercentage {
    LengthPercentage::Px(f32::from(px.0))
}

fn length_auto(px: Px) -> LengthPercentageAuto {
    LengthPercentageAuto::Px(f32::from(px.0))
}

fn dimension(value: StyleDimension) -> Dimension {
    match value {
        StyleDimension::Px(px) => Dimension::Px(f32::from(px.0)),
        StyleDimension::Full => Dimension::Percent(1.0),
        StyleDimension::Fraction(scaled) => Dimension::Percent(f32::from(scaled) / 10_000.0),
        StyleDimension::Auto => Dimension::Auto,
    }
}

const fn display_kind(display: DisplayKind) -> Display {
    match display {
        DisplayKind::Flex => Display::Flex,
        DisplayKind::Block => Display::Block,
        DisplayKind::Grid => Display::Grid,
        DisplayKind::None => Display::None,
    }
}

const fn flex_direction(direction: FlexDir) -> FlexDirection {
    match direction {
        FlexDir::Row => FlexDirection::Row,
        FlexDir::Column => FlexDirection::Column,
        FlexDir::RowReverse => FlexDirection::RowReverse,
        FlexDir::ColumnReverse => FlexDirection::ColumnReverse,
    }
}

const fn align_items(align: AlignKind) -> AlignItems {
    match align {
        AlignKind::Start => AlignItems::Start,
        AlignKind::Center | AlignKind::Between | AlignKind::Around => AlignItems::Center,
        AlignKind::End => AlignItems::End,
        AlignKind::Stretch => AlignItems::Stretch,
    }
}

const fn justify_content(justify: JustifyKind) -> JustifyContent {
    match justify {
        JustifyKind::Start => JustifyContent::Start,
        JustifyKind::Center => JustifyContent::Center,
        JustifyKind::End => JustifyContent::End,
        JustifyKind::Stretch => JustifyContent::Stretch,
        JustifyKind::Between => JustifyContent::SpaceBetween,
        JustifyKind::Around => JustifyContent::SpaceAround,
        JustifyKind::Evenly => JustifyContent::SpaceEvenly,
    }
}
