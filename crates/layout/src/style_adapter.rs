//! Structural style atom adapter for `LayoutStyle`.

#![allow(clippy::redundant_pub_crate)]

use sigil_style::{
    AlignKind, Dimension as StyleDimension, DisplayKind, FlexDir, FlexWrapKind, JustifyKind,
    OverflowKind, Px, StyleAtom,
};

use crate::{
    LayoutStyle,
    style_data::{
        AlignItems, Dimension, Display, FlexDirection, FlexWrap, JustifyContent, LengthPercentage,
        LengthPercentageAuto, Overflow, StyleData,
    },
};

/// Apply a slice of [`StyleAtom`] values onto a [`LayoutStyle`].
///
/// Used internally by the layout engine when resolving a node's
/// `StyleBinding`; exposed at the crate root so tests can verify the
/// round-trip from atoms to the resolved [`StyleData`] without going
/// through the full engine compute path.
pub fn apply_style_atoms(atoms: &[StyleAtom], style: &mut LayoutStyle) {
    for atom in atoms {
        apply_style_atom(*atom, style);
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "exhaustive match on StyleAtom — the long no-op arm chain is intentional; a wildcard would silently swallow new atom kinds"
)]
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
        StyleAtom::GapX(gap) => set_gap_x(style, gap),
        StyleAtom::GapY(gap) => set_gap_y(style, gap),
        StyleAtom::Width(width) => set_width(style, width),
        StyleAtom::Height(height) => set_height(style, height),
        StyleAtom::MinWidth(width) => set_min_width(style, width),
        StyleAtom::MaxWidth(width) => set_max_width(style, width),
        StyleAtom::MinHeight(height) => set_min_height(style, height),
        StyleAtom::MaxHeight(height) => set_max_height(style, height),
        StyleAtom::Display(display) => set_display(style, display),
        StyleAtom::GridColumns(n) => set_grid_columns(style, n),
        StyleAtom::GridRows(n) => set_grid_rows(style, n),
        StyleAtom::FlexDirection(direction) => set_flex_direction(style, direction),
        StyleAtom::FlexWrap(wrap) => set_flex_wrap(style, wrap),
        StyleAtom::AlignItems(align) => set_align_items(style, align),
        StyleAtom::JustifyContent(justify) => set_justify_content(style, justify),
        StyleAtom::FlexGrow(grow) => set_flex_grow(style, grow),
        StyleAtom::FlexShrink(shrink) => set_flex_shrink(style, shrink),
        StyleAtom::OverflowX(kind) => set_overflow_x(style, kind),
        StyleAtom::OverflowY(kind) => set_overflow_y(style, kind),
        // Border width is box-model-affecting: the solvers consume it
        // as edge thickness (DR-057 D3). Colour halves are paint-only.
        StyleAtom::BorderWidth(width) => set_border(style, width),
        StyleAtom::BorderColor(_)
        | StyleAtom::BorderThemeRef(_)
        | StyleAtom::BorderToneStep(_)
        | StyleAtom::BackgroundColor(_)
        | StyleAtom::BackgroundThemeRef(_)
        | StyleAtom::TextColor(_)
        | StyleAtom::TextThemeRef(_)
        | StyleAtom::BorderRadius(_)
        | StyleAtom::Shadow(_)
        | StyleAtom::Opacity(_)
        // Typography is resolved in `text_style.rs::apply_text_binding`;
        // keep cache-key coverage in sync with that function.
        | StyleAtom::FontSize(_)
        | StyleAtom::FontWeight(_)
        | StyleAtom::FontFamily(_)
        | StyleAtom::LineHeight(_)
        | StyleAtom::Transition(_)
        | StyleAtom::TransitionDuration(_)
        | StyleAtom::UnknownThemeRef(_)
        | StyleAtom::BgToneStep(_)
        | StyleAtom::FgToneStep(_)
        | StyleAtom::FgOnSolid => {}
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

fn set_gap_x(layout: &mut LayoutStyle, gap: Px) {
    mutate_layout(layout, |style| {
        style.gap.width = length(gap);
    });
}

fn set_gap_y(layout: &mut LayoutStyle, gap: Px) {
    mutate_layout(layout, |style| {
        style.gap.height = length(gap);
    });
}

fn set_width(layout: &mut LayoutStyle, width: StyleDimension) {
    mutate_layout(layout, |style| style.size.width = dimension(width));
}

fn set_height(layout: &mut LayoutStyle, height: StyleDimension) {
    mutate_layout(layout, |style| style.size.height = dimension(height));
}

fn set_min_width(layout: &mut LayoutStyle, width: StyleDimension) {
    mutate_layout(layout, |style| style.min_size.width = dimension(width));
}

fn set_max_width(layout: &mut LayoutStyle, width: StyleDimension) {
    mutate_layout(layout, |style| style.max_size.width = dimension(width));
}

fn set_min_height(layout: &mut LayoutStyle, height: StyleDimension) {
    mutate_layout(layout, |style| style.min_size.height = dimension(height));
}

fn set_max_height(layout: &mut LayoutStyle, height: StyleDimension) {
    mutate_layout(layout, |style| style.max_size.height = dimension(height));
}

fn set_border(layout: &mut LayoutStyle, width: Px) {
    mutate_layout(layout, |style| {
        style.border.left = length(width);
        style.border.right = length(width);
        style.border.top = length(width);
        style.border.bottom = length(width);
    });
}

fn set_display(layout: &mut LayoutStyle, display: DisplayKind) {
    mutate_layout(layout, |style| style.display = display_kind(display));
}

fn set_grid_columns(layout: &mut LayoutStyle, n: u16) {
    mutate_layout(layout, |style| {
        style.display = Display::Grid;
        style.grid_template_columns = vec![crate::TrackSize::Fr(1.0); n as usize];
    });
}

fn set_grid_rows(layout: &mut LayoutStyle, n: u16) {
    mutate_layout(layout, |style| {
        style.display = Display::Grid;
        style.grid_template_rows = vec![crate::TrackSize::Auto; n as usize];
    });
}

fn set_flex_direction(layout: &mut LayoutStyle, direction: FlexDir) {
    mutate_layout(layout, |style| {
        style.flex_direction = flex_direction(direction);
    });
}

fn set_flex_wrap(layout: &mut LayoutStyle, wrap: FlexWrapKind) {
    mutate_layout(layout, |style| {
        style.flex_wrap = match wrap {
            FlexWrapKind::NoWrap => FlexWrap::NoWrap,
            FlexWrapKind::Wrap => FlexWrap::Wrap,
        };
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

fn set_overflow_x(layout: &mut LayoutStyle, kind: OverflowKind) {
    mutate_layout(layout, |style| style.overflow.width = overflow(kind));
}

fn set_overflow_y(layout: &mut LayoutStyle, kind: OverflowKind) {
    mutate_layout(layout, |style| style.overflow.height = overflow(kind));
}

const fn overflow(kind: OverflowKind) -> Overflow {
    match kind {
        OverflowKind::Visible => Overflow::Visible,
        OverflowKind::Hidden => Overflow::Hidden,
        // CSS `auto` currently aliases Scroll (documented on the atom).
        OverflowKind::Scroll | OverflowKind::Auto => Overflow::Scroll,
    }
}

fn mutate_layout(layout: &mut LayoutStyle, mutate: impl FnOnce(&mut StyleData)) {
    mutate(layout.data_mut());
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
