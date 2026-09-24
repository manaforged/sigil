use crate::hal::{HitPosition, TextAffinity};
use crate::layout::style_data::{Dimension, LengthPercentage, Rect, Size, StyleData};
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
        Dimension::Text(length) => Dimension::Px(length.px()),
        other => other,
    }
}

fn length(value: LengthPercentage) -> LengthPercentage {
    match value {
        LengthPercentage::Text(length) => LengthPercentage::Px(length.px()),
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

pub(crate) fn resolve_style(mut style: StyleData) -> StyleData {
    style.flex_basis = dimension(style.flex_basis);
    style.size = dimensions(style.size);
    style.min_size = dimensions(style.min_size);
    style.max_size = dimensions(style.max_size);
    style.gap = Size::new(length(style.gap.width), length(style.gap.height));
    style.padding = edges(style.padding);
    style.border = edges(style.border);
    style
}
