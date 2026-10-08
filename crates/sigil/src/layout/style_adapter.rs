#![allow(clippy::redundant_pub_crate)]
#![cfg_attr(not(any(test, feature = "kittest")), allow(unreachable_pub))]

use crate::style::{
    AlignKind, Dimension as StyleDimension, DisplayKind, FlexDir, FlexWrapKind, JustifyKind,
    OverflowKind, Px, StyleAtom,
};

use crate::layout::{
    LayoutStyle,
    style_data::{
        AlignItems, Dimension, Display, FlexDirection, FlexWrap, JustifyContent, LengthPercentage,
        LengthPercentageAuto, Overflow, StyleData,
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
        StyleAtom::Gap(gap) => style.set_gap(gap),
        StyleAtom::GapX(gap) => style.set_gap_x(gap),
        StyleAtom::GapY(gap) => style.set_gap_y(gap),
        StyleAtom::Width(width) => style.set_width(width),
        StyleAtom::Height(height) => style.set_height(height),
        StyleAtom::MinWidth(width) => style.set_min_width(width),
        StyleAtom::MaxWidth(width) => style.set_max_width(width),
        StyleAtom::MinHeight(height) => style.set_min_height(height),
        StyleAtom::MaxHeight(height) => style.set_max_height(height),
        StyleAtom::Display(display) => style.set_display(display),
        StyleAtom::GridColumns(n) => style.set_grid_columns(n),
        StyleAtom::GridRows(n) => style.set_grid_rows(n),
        StyleAtom::FlexDirection(direction) => style.set_flex_direction(direction),
        StyleAtom::FlexWrap(wrap) => style.set_flex_wrap(wrap),
        StyleAtom::AlignItems(align) => style.set_align_items(align),
        StyleAtom::JustifyContent(justify) => style.set_justify_content(justify),
        StyleAtom::FlexGrow(grow) => style.set_flex_grow(grow),
        StyleAtom::FlexShrink(shrink) => style.set_flex_shrink(shrink),
        StyleAtom::OverflowX(kind) => style.set_overflow_x(kind),
        StyleAtom::OverflowY(kind) => style.set_overflow_y(kind),
        StyleAtom::BorderWidth(width) => style.set_border(width),
        StyleAtom::BorderTop(width) => style.set_border_top(width),
        StyleAtom::BorderRight(width) => style.set_border_right(width),
        StyleAtom::BorderBottom(width) => style.set_border_bottom(width),
        StyleAtom::BorderLeft(width) => style.set_border_left(width),
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

impl LayoutStyle {
    fn set_gap(&mut self, gap: Px) {
        mutate_layout(self, |style| {
            style.gap.width = length(gap);
            style.gap.height = length(gap);
        });
    }

    fn set_gap_x(&mut self, gap: Px) {
        mutate_layout(self, |style| {
            style.gap.width = length(gap);
        });
    }

    fn set_gap_y(&mut self, gap: Px) {
        mutate_layout(self, |style| {
            style.gap.height = length(gap);
        });
    }

    fn set_width(&mut self, width: StyleDimension) {
        mutate_layout(self, |style| style.size.width = dimension(width));
    }

    fn set_height(&mut self, height: StyleDimension) {
        mutate_layout(self, |style| style.size.height = dimension(height));
    }

    fn set_min_width(&mut self, width: StyleDimension) {
        mutate_layout(self, |style| style.min_size.width = dimension(width));
    }

    fn set_max_width(&mut self, width: StyleDimension) {
        mutate_layout(self, |style| style.max_size.width = dimension(width));
    }

    fn set_min_height(&mut self, height: StyleDimension) {
        mutate_layout(self, |style| style.min_size.height = dimension(height));
    }

    fn set_max_height(&mut self, height: StyleDimension) {
        mutate_layout(self, |style| style.max_size.height = dimension(height));
    }

    fn set_border(&mut self, width: Px) {
        mutate_layout(self, |style| {
            style.border.left = length(width);
            style.border.right = length(width);
            style.border.top = length(width);
            style.border.bottom = length(width);
        });
    }

    fn set_border_top(&mut self, width: Px) {
        mutate_layout(self, |style| style.border.top = length(width));
    }

    fn set_border_right(&mut self, width: Px) {
        mutate_layout(self, |style| style.border.right = length(width));
    }

    fn set_border_bottom(&mut self, width: Px) {
        mutate_layout(self, |style| style.border.bottom = length(width));
    }

    fn set_border_left(&mut self, width: Px) {
        mutate_layout(self, |style| style.border.left = length(width));
    }

    fn set_display(&mut self, display: DisplayKind) {
        mutate_layout(self, |style| style.display = display_kind(display));
    }

    fn set_grid_columns(&mut self, n: u16) {
        mutate_layout(self, |style| {
            style.display = Display::Grid;
            style.grid_template_columns = vec![crate::layout::TrackSize::Fr(1.0); n as usize];
        });
    }

    fn set_grid_rows(&mut self, n: u16) {
        mutate_layout(self, |style| {
            style.display = Display::Grid;
            style.grid_template_rows = vec![crate::layout::TrackSize::Auto; n as usize];
        });
    }

    fn set_flex_direction(&mut self, direction: FlexDir) {
        mutate_layout(self, |style| {
            style.flex_direction = flex_direction(direction);
        });
    }

    fn set_flex_wrap(&mut self, wrap: FlexWrapKind) {
        mutate_layout(self, |style| {
            style.flex_wrap = match wrap {
                FlexWrapKind::NoWrap => FlexWrap::NoWrap,
                FlexWrapKind::Wrap => FlexWrap::Wrap,
            };
        });
    }

    fn set_align_items(&mut self, align: AlignKind) {
        mutate_layout(self, |style| {
            style.align_items = Some(align_items(align));
        });
    }

    fn set_justify_content(&mut self, justify: JustifyKind) {
        mutate_layout(self, |style| {
            style.justify_content = Some(justify_content(justify));
        });
    }

    fn set_flex_grow(&mut self, grow: u16) {
        mutate_layout(self, |style| {
            style.flex_grow = f32::from(grow) / crate::style::atom::FACTOR_SCALE;
        });
    }

    fn set_flex_shrink(&mut self, shrink: u16) {
        mutate_layout(self, |style| {
            style.flex_shrink = f32::from(shrink) / crate::style::atom::FACTOR_SCALE;
        });
    }

    fn set_overflow_x(&mut self, kind: OverflowKind) {
        mutate_layout(self, |style| style.overflow.width = overflow(kind));
    }

    fn set_overflow_y(&mut self, kind: OverflowKind) {
        mutate_layout(self, |style| style.overflow.height = overflow(kind));
    }
}

const fn overflow(kind: OverflowKind) -> Overflow {
    match kind {
        OverflowKind::Visible => Overflow::Visible,
        OverflowKind::Hidden => Overflow::Hidden,
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
