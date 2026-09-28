use crate::hal::{HitPosition, TextAffinity};
use crate::layout::style_data::{
    Dimension, LengthPercentage, LengthPercentageAuto, Rect, Size, StyleData, TrackSize,
};
use crate::layout::text_geometry::with_text_geometry;
use crate::style::{TextLineHeight, TextStyle};

const ADVANCE_SAMPLE: &str = "abcdefghijklmnopqrstuvwxyz0123456789";

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextLength {
    Rows(f32),
    Cols(f32),
    Em(f32),
}

#[must_use]
pub const fn rows(count: f32) -> TextLength {
    TextLength::Rows(count)
}

#[must_use]
pub const fn cols(count: f32) -> TextLength {
    TextLength::Cols(count)
}

#[must_use]
pub const fn em(count: f32) -> TextLength {
    TextLength::Em(count)
}

impl From<TextLength> for Dimension {
    fn from(length: TextLength) -> Self {
        Self::Text(length)
    }
}

impl From<TextLength> for LengthPercentage {
    fn from(length: TextLength) -> Self {
        Self::Text(length)
    }
}

impl TextLength {
    #[must_use]
    pub fn px(self) -> f32 {
        match self {
            Self::Rows(count) => count * row(),
            Self::Cols(count) => count * col(),
            Self::Em(count) => count * font_size(),
        }
    }
}

pub trait IntoPx {
    fn into_px(self) -> f32;
}

impl IntoPx for f32 {
    fn into_px(self) -> f32 {
        self
    }
}

impl IntoPx for TextLength {
    fn into_px(self) -> f32 {
        self.px()
    }
}

fn cell() -> Option<f32> {
    crate::style::cell_size().filter(|cell| *cell > 0.0)
}

fn font_size() -> f32 {
    cell().unwrap_or_else(|| TextStyle::default().font_size)
}

fn row() -> f32 {
    cell().unwrap_or_else(|| {
        let style = TextStyle::default();
        with_text_geometry(|geometry| geometry.caret("", &style, None, downstream(0)).height)
            .filter(|height| *height > 0.0)
            .unwrap_or(match style.line_height {
                TextLineHeight::FontSizeRelative(factor) => style.font_size * factor,
                TextLineHeight::Absolute(value) => value,
            })
    })
}

#[allow(
    clippy::cast_precision_loss,
    reason = "the sample length is a small constant"
)]
fn col() -> f32 {
    cell().unwrap_or_else(|| {
        let style = TextStyle::default();
        let end = downstream(ADVANCE_SAMPLE.len());
        with_text_geometry(|geometry| geometry.caret(ADVANCE_SAMPLE, &style, None, end).x)
            .map(|width| width / ADVANCE_SAMPLE.len() as f32)
            .filter(|advance| *advance > 0.0)
            .unwrap_or(style.font_size * 0.5)
    })
}

const fn downstream(byte_index: usize) -> HitPosition {
    HitPosition {
        byte_index,
        affinity: TextAffinity::Downstream,
    }
}

fn dimension(value: Dimension) -> Dimension {
    match value {
        Dimension::Text(length) => finite_dimension(length.px()),
        Dimension::Px(px) => finite_dimension(px),
        Dimension::Percent(pct) if !pct.is_finite() => Dimension::Auto,
        other => other,
    }
}

const fn finite_dimension(px: f32) -> Dimension {
    if px.is_finite() {
        Dimension::Px(px)
    } else {
        Dimension::Auto
    }
}

fn length(value: LengthPercentage) -> LengthPercentage {
    let px = match value {
        LengthPercentage::Text(length) => length.px(),
        LengthPercentage::Px(px) => px,
        LengthPercentage::Percent(pct) if pct.is_finite() => return value,
        LengthPercentage::Percent(_) => 0.0,
    };
    LengthPercentage::Px(if px.is_finite() { px } else { 0.0 })
}

const fn length_auto(
    value: LengthPercentageAuto,
    fallback: LengthPercentageAuto,
) -> LengthPercentageAuto {
    match value {
        LengthPercentageAuto::Px(v) | LengthPercentageAuto::Percent(v) if !v.is_finite() => {
            fallback
        }
        other => other,
    }
}

fn dimensions(size: Size<Dimension>) -> Size<Dimension> {
    Size::new(dimension(size.width), dimension(size.height))
}

fn edges(rect: Rect<LengthPercentage>) -> Rect<LengthPercentage> {
    Rect {
        left: length(rect.left),
        right: length(rect.right),
        top: length(rect.top),
        bottom: length(rect.bottom),
    }
}

const fn auto_edges(
    rect: Rect<LengthPercentageAuto>,
    fallback: LengthPercentageAuto,
) -> Rect<LengthPercentageAuto> {
    Rect {
        left: length_auto(rect.left, fallback),
        right: length_auto(rect.right, fallback),
        top: length_auto(rect.top, fallback),
        bottom: length_auto(rect.bottom, fallback),
    }
}

const fn factor(value: f32, initial: f32) -> f32 {
    if value.is_finite() && value >= 0.0 {
        value
    } else {
        initial
    }
}

const fn track(value: TrackSize) -> TrackSize {
    let valid = match value {
        TrackSize::Px(v) => v.is_finite(),
        TrackSize::Fr(v) => v.is_finite() && v >= 0.0,
        TrackSize::MinMax(lo, hi) => lo.is_finite() && hi.is_finite(),
        TrackSize::Auto => true,
    };
    if valid { value } else { TrackSize::Auto }
}

pub(crate) fn resolve_style(mut style: StyleData) -> StyleData {
    let initial = StyleData::default();
    style.flex_grow = factor(style.flex_grow, initial.flex_grow);
    style.flex_shrink = factor(style.flex_shrink, initial.flex_shrink);
    style.flex_basis = dimension(style.flex_basis);
    style.size = dimensions(style.size);
    style.min_size = dimensions(style.min_size);
    style.max_size = dimensions(style.max_size);
    style.gap = Size::new(length(style.gap.width), length(style.gap.height));
    style.padding = edges(style.padding);
    style.border = edges(style.border);
    style.margin = auto_edges(style.margin, LengthPercentageAuto::Px(0.0));
    style.inset = auto_edges(style.inset, LengthPercentageAuto::Auto);
    for list in [
        &mut style.grid_template_columns,
        &mut style.grid_template_rows,
    ] {
        for size in list.iter_mut() {
            *size = track(*size);
        }
    }
    style
}
