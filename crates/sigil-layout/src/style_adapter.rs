//! Structural style atom adapter for `LayoutStyle`.

#![allow(clippy::redundant_pub_crate)]

use sigil_style::{AlignKind, Dimension, DisplayKind, FlexDir, JustifyKind, Px, StyleAtom};
use taffy::prelude::{
    AlignItems as TaffyAlignItems, Dimension as TaffyDimension, Display as TaffyDisplay,
    FlexDirection as TaffyFlexDirection, JustifyContent as TaffyJustifyContent, LengthPercentage,
    LengthPercentageAuto, Style as TaffyStyle,
};

use crate::LayoutStyle;

pub(crate) fn apply_style_atoms(atoms: &[StyleAtom], style: &mut LayoutStyle) {
    for atom in atoms {
        apply_style_atom(*atom, style);
    }
}

fn apply_style_atom(atom: StyleAtom, style: &mut LayoutStyle) {
    match atom {
        StyleAtom::PaddingAll(_)
        | StyleAtom::PaddingX(_)
        | StyleAtom::PaddingY(_)
        | StyleAtom::PaddingTop(_)
        | StyleAtom::PaddingRight(_)
        | StyleAtom::PaddingBottom(_)
        | StyleAtom::PaddingLeft(_) => apply_padding(atom, style),
        StyleAtom::MarginAll(_)
        | StyleAtom::MarginX(_)
        | StyleAtom::MarginY(_)
        | StyleAtom::MarginTop(_)
        | StyleAtom::MarginRight(_)
        | StyleAtom::MarginBottom(_)
        | StyleAtom::MarginLeft(_) => apply_margin(atom, style),
        StyleAtom::Gap(gap) => set_gap(style, gap),
        StyleAtom::Width(width) => set_width(style, width),
        StyleAtom::Height(height) => set_height(style, height),
        StyleAtom::MinWidth(width) => set_min_width(style, width),
        StyleAtom::MaxWidth(width) => set_max_width(style, width),
        StyleAtom::Display(display) => set_display(style, display),
        StyleAtom::FlexDirection(direction) => set_flex_direction(style, direction),
        StyleAtom::AlignItems(align) => set_align_items(style, align),
        StyleAtom::JustifyContent(justify) => set_justify_content(style, justify),
        StyleAtom::FlexGrow(grow) => set_flex_grow(style, grow),
        StyleAtom::FlexShrink(shrink) => set_flex_shrink(style, shrink),
        StyleAtom::BackgroundColor(_)
        | StyleAtom::BackgroundThemeRef(_)
        | StyleAtom::TextColor(_)
        | StyleAtom::TextThemeRef(_)
        | StyleAtom::BorderRadius(_)
        | StyleAtom::Opacity(_)
        | StyleAtom::FontSize(_)
        | StyleAtom::FontWeight(_)
        | StyleAtom::LineHeight(_)
        | StyleAtom::Transition(_)
        | StyleAtom::TransitionDuration(_)
        | StyleAtom::UnknownThemeRef(_) => {}
    }
}

fn apply_padding(atom: StyleAtom, layout: &mut LayoutStyle) {
    mutate_layout(layout, |style| match atom {
        StyleAtom::PaddingAll(px) => {
            style.padding.left = length(px);
            style.padding.right = length(px);
            style.padding.top = length(px);
            style.padding.bottom = length(px);
        }
        StyleAtom::PaddingX(px) => {
            style.padding.left = length(px);
            style.padding.right = length(px);
        }
        StyleAtom::PaddingY(px) => {
            style.padding.top = length(px);
            style.padding.bottom = length(px);
        }
        StyleAtom::PaddingTop(px) => style.padding.top = length(px),
        StyleAtom::PaddingRight(px) => style.padding.right = length(px),
        StyleAtom::PaddingBottom(px) => style.padding.bottom = length(px),
        StyleAtom::PaddingLeft(px) => style.padding.left = length(px),
        _ => {}
    });
}

fn apply_margin(atom: StyleAtom, layout: &mut LayoutStyle) {
    mutate_layout(layout, |style| match atom {
        StyleAtom::MarginAll(px) => {
            style.margin.left = length_auto(px);
            style.margin.right = length_auto(px);
            style.margin.top = length_auto(px);
            style.margin.bottom = length_auto(px);
        }
        StyleAtom::MarginX(px) => {
            style.margin.left = length_auto(px);
            style.margin.right = length_auto(px);
        }
        StyleAtom::MarginY(px) => {
            style.margin.top = length_auto(px);
            style.margin.bottom = length_auto(px);
        }
        StyleAtom::MarginTop(px) => style.margin.top = length_auto(px),
        StyleAtom::MarginRight(px) => style.margin.right = length_auto(px),
        StyleAtom::MarginBottom(px) => style.margin.bottom = length_auto(px),
        StyleAtom::MarginLeft(px) => style.margin.left = length_auto(px),
        _ => {}
    });
}

fn set_gap(layout: &mut LayoutStyle, gap: Px) {
    mutate_layout(layout, |style| {
        style.gap.width = length(gap);
        style.gap.height = length(gap);
    });
}

fn set_width(layout: &mut LayoutStyle, width: Dimension) {
    mutate_layout(layout, |style| style.size.width = dimension(width));
}

fn set_height(layout: &mut LayoutStyle, height: Dimension) {
    mutate_layout(layout, |style| style.size.height = dimension(height));
}

fn set_min_width(layout: &mut LayoutStyle, width: Dimension) {
    mutate_layout(layout, |style| style.min_size.width = dimension(width));
}

fn set_max_width(layout: &mut LayoutStyle, width: Dimension) {
    mutate_layout(layout, |style| style.max_size.width = dimension(width));
}

fn set_display(layout: &mut LayoutStyle, display: DisplayKind) {
    mutate_layout(layout, |style| style.display = display_kind(display));
}

fn set_flex_direction(layout: &mut LayoutStyle, direction: FlexDir) {
    mutate_layout(layout, |style| {
        style.flex_direction = flex_direction(direction);
    });
}

fn set_align_items(layout: &mut LayoutStyle, align: AlignKind) {
    mutate_layout(layout, |style| {
        style.align_items = Some(align_items(align));
    });
}

fn set_justify_content(layout: &mut LayoutStyle, justify: JustifyKind) {
    mutate_layout(layout, |style| {
        style.justify_content = Some(justify_content(justify));
    });
}

fn set_flex_grow(layout: &mut LayoutStyle, grow: u16) {
    mutate_layout(layout, |style| style.flex_grow = f32::from(grow) / 1000.0);
}

fn set_flex_shrink(layout: &mut LayoutStyle, shrink: u16) {
    mutate_layout(layout, |style| {
        style.flex_shrink = f32::from(shrink) / 1000.0;
    });
}

fn mutate_layout(layout: &mut LayoutStyle, mutate: impl FnOnce(&mut TaffyStyle)) {
    let current = core::mem::take(layout);
    *layout = current.with_taffy(mutate);
}

fn length(px: Px) -> LengthPercentage {
    LengthPercentage::length(f32::from(px.0))
}

fn length_auto(px: Px) -> LengthPercentageAuto {
    LengthPercentageAuto::length(f32::from(px.0))
}

fn dimension(value: Dimension) -> TaffyDimension {
    match value {
        Dimension::Px(px) => TaffyDimension::length(f32::from(px.0)),
        Dimension::Full => TaffyDimension::percent(1.0),
        Dimension::Auto => TaffyDimension::auto(),
    }
}

const fn display_kind(display: DisplayKind) -> TaffyDisplay {
    match display {
        DisplayKind::Flex => TaffyDisplay::Flex,
        DisplayKind::Block => TaffyDisplay::Block,
        DisplayKind::None => TaffyDisplay::None,
    }
}

const fn flex_direction(direction: FlexDir) -> TaffyFlexDirection {
    match direction {
        FlexDir::Row => TaffyFlexDirection::Row,
        FlexDir::Column => TaffyFlexDirection::Column,
        FlexDir::RowReverse => TaffyFlexDirection::RowReverse,
        FlexDir::ColumnReverse => TaffyFlexDirection::ColumnReverse,
    }
}

const fn align_items(align: AlignKind) -> TaffyAlignItems {
    match align {
        AlignKind::Start => TaffyAlignItems::Start,
        AlignKind::Center | AlignKind::Between | AlignKind::Around => TaffyAlignItems::Center,
        AlignKind::End => TaffyAlignItems::End,
        AlignKind::Stretch => TaffyAlignItems::Stretch,
    }
}

const fn justify_content(justify: JustifyKind) -> TaffyJustifyContent {
    match justify {
        JustifyKind::Start => TaffyJustifyContent::Start,
        JustifyKind::Center => TaffyJustifyContent::Center,
        JustifyKind::End => TaffyJustifyContent::End,
        JustifyKind::Stretch => TaffyJustifyContent::Stretch,
        JustifyKind::Between => TaffyJustifyContent::SpaceBetween,
        JustifyKind::Around => TaffyJustifyContent::SpaceAround,
    }
}
