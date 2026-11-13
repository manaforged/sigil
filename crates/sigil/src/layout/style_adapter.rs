#![allow(clippy::redundant_pub_crate)]

use crate::style::StyleAtom;
use crate::style::atom::{FACTOR_SCALE, decode_atom};

use crate::layout::{
    LayoutStyle,
    style_data::{Axes, Display, Edges, TrackSize},
};

#[doc(hidden)]
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
        StyleAtom::PaddingAll(px) => s.padding = Edges::splat(edge_px(px)),
        StyleAtom::PaddingX(px) => {
            (s.padding.left, s.padding.right) = (edge_px(px), edge_px(px));
        }
        StyleAtom::PaddingY(px) => {
            (s.padding.top, s.padding.bottom) = (edge_px(px), edge_px(px));
        }
        StyleAtom::PaddingTop(px) => s.padding.top = edge_px(px),
        StyleAtom::PaddingRight(px) => s.padding.right = edge_px(px),
        StyleAtom::PaddingBottom(px) => s.padding.bottom = edge_px(px),
        StyleAtom::PaddingLeft(px) => s.padding.left = edge_px(px),
        StyleAtom::MarginAll(px) => s.margin = Edges::splat(edge_px(px)),
        StyleAtom::MarginX(px) => {
            (s.margin.left, s.margin.right) = (edge_px(px), edge_px(px));
        }
        StyleAtom::MarginY(px) => {
            (s.margin.top, s.margin.bottom) = (edge_px(px), edge_px(px));
        }
        StyleAtom::MarginTop(px) => s.margin.top = edge_px(px),
        StyleAtom::MarginRight(px) => s.margin.right = edge_px(px),
        StyleAtom::MarginBottom(px) => s.margin.bottom = edge_px(px),
        StyleAtom::MarginLeft(px) => s.margin.left = edge_px(px),
        StyleAtom::Gap(gap) => s.gap = Axes::splat(edge_px(gap)),
        StyleAtom::GapX(gap) => s.gap.width = edge_px(gap),
        StyleAtom::GapY(gap) => s.gap.height = edge_px(gap),
        StyleAtom::Width(kind, value) => s.size.width = decode_atom(kind, value),
        StyleAtom::Height(kind, value) => s.size.height = decode_atom(kind, value),
        StyleAtom::MinWidth(kind, value) => s.min_size.width = decode_atom(kind, value),
        StyleAtom::MaxWidth(kind, value) => s.max_size.width = decode_atom(kind, value),
        StyleAtom::MinHeight(kind, value) => s.min_size.height = decode_atom(kind, value),
        StyleAtom::MaxHeight(kind, value) => s.max_size.height = decode_atom(kind, value),
        StyleAtom::Display(display) => s.display = display,
        StyleAtom::GridColumns(n) => {
            s.display = Display::Grid;
            s.grid_template_columns = vec![TrackSize::Fr(1.0); n as usize];
        }
        StyleAtom::GridRows(n) => {
            s.display = Display::Grid;
            s.grid_template_rows = vec![TrackSize::Auto; n as usize];
        }
        StyleAtom::FlexDirection(direction) => s.flex_direction = direction,
        StyleAtom::FlexWrap(wrap) => s.flex_wrap = wrap,
        StyleAtom::AlignItems(align) => s.align_items = Some(align),
        StyleAtom::JustifyContent(justify) => s.justify_content = Some(justify),
        StyleAtom::FlexGrow(grow) => s.flex_grow = f32::from(grow) / FACTOR_SCALE,
        StyleAtom::FlexShrink(shrink) => s.flex_shrink = f32::from(shrink) / FACTOR_SCALE,
        StyleAtom::OverflowX(kind) => s.overflow.width = kind,
        StyleAtom::OverflowY(kind) => s.overflow.height = kind,
        StyleAtom::BorderWidth(width) => s.border = Edges::splat(edge_px(width)),
        StyleAtom::BorderTop(width) => s.border.top = edge_px(width),
        StyleAtom::BorderRight(width) => s.border.right = edge_px(width),
        StyleAtom::BorderBottom(width) => s.border.bottom = edge_px(width),
        StyleAtom::BorderLeft(width) => s.border.left = edge_px(width),
        StyleAtom::BorderColor(_)
        | StyleAtom::BorderToneStep(_)
        | StyleAtom::BackgroundColor(_)
        | StyleAtom::BackgroundGradient(_)
        | StyleAtom::TextColor(_)
        | StyleAtom::BorderRadius(_)
        | StyleAtom::Radius(_)
        | StyleAtom::Shadow(_)
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
        | StyleAtom::TransitionEase(_)
        | StyleAtom::TransitionDelay(_)
        | StyleAtom::Animation(_)
        | StyleAtom::Paint(_)
        | StyleAtom::ClipPath(_)
        | StyleAtom::Space(_)
        | StyleAtom::Mask(_)
        | StyleAtom::MixBlend(_)
        | StyleAtom::Filter(_)
        | StyleAtom::DropShadowColor(_)
        | StyleAtom::TextShadowColor(_)
        | StyleAtom::BgToneStep(_)
        | StyleAtom::FgToneStep(_)
        | StyleAtom::FgOnSolid => {}
        #[cfg(feature = "kittest")]
        StyleAtom::Transform(_) => {}
    }
}

fn edge_px(px: i16) -> crate::layout::Length {
    crate::layout::Length::Px(f32::from(px))
}
