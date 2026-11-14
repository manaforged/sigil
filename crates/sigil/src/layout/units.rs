use crate::hal::{HitPosition, TextAffinity};
use crate::layout::snap::cell;
use crate::layout::style_data::{Length, LengthPercentage, StyleData, TrackSize};
use crate::layout::text_geometry::with_text_geometry;
use crate::style::{TextLineHeight, TextStyle};

const ADVANCE_SAMPLE: &str = "abcdefghijklmnopqrstuvwxyz0123456789";

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TextLength {
    Rows(f32),
    Cols(f32),
    Em(f32),
}

/// Height of `count` text lines.
#[must_use]
pub const fn rows(count: f32) -> Length {
    Length::Rows(count)
}

/// Width of `count` character cells.
#[must_use]
pub const fn cols(count: f32) -> Length {
    Length::Cols(count)
}

/// `count` times the font size.
#[must_use]
pub const fn em(count: f32) -> Length {
    Length::Em(count)
}

impl Length {
    /// Returns whether the length takes a share of its container: `Fill` or a percentage.
    pub(crate) const fn fills(self) -> bool {
        matches!(self, Self::Fill | Self::Percent(_))
    }

    pub(crate) fn px(self) -> Option<f32> {
        match self {
            Self::Px(px) => Some(px),
            other => other.text_px(),
        }
    }

    pub(crate) fn text_px(self) -> Option<f32> {
        match self {
            Self::Rows(count) => Some(TextLength::Rows(count).px()),
            Self::Cols(count) => Some(TextLength::Cols(count).px()),
            Self::Em(count) => Some(TextLength::Em(count).px()),
            Self::Px(_) | Self::Percent(_) | Self::Auto | Self::Fill => None,
        }
    }
}

impl From<TextLength> for LengthPercentage {
    fn from(length: TextLength) -> Self {
        Self::Text(length)
    }
}

impl TextLength {
    #[must_use]
    pub(crate) fn px(self) -> f32 {
        match self {
            Self::Rows(count) => count * row(),
            Self::Cols(count) => count * col(),
            Self::Em(count) => count * font_size(),
        }
    }
}

pub(crate) fn px_or_zero(length: impl Into<Length>) -> f32 {
    length.into().px().unwrap_or(0.0)
}

pub(crate) fn positive_px(length: impl Into<Length>) -> Option<f32> {
    length.into().px().filter(|&px| px.is_finite() && px > 0.0)
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

fn col() -> f32 {
    cell().unwrap_or_else(|| {
        let style = TextStyle::default();
        advance(&style).unwrap_or(style.font_size * 0.5)
    })
}

fn advance(style: &TextStyle) -> Option<f32> {
    let count = f32::from(u16::try_from(ADVANCE_SAMPLE.len()).ok()?);
    let end = downstream(ADVANCE_SAMPLE.len());
    with_text_geometry(|geometry| geometry.caret(ADVANCE_SAMPLE, style, None, end).x)
        .map(|width| width / count)
        .filter(|advance| *advance > 0.0)
}

#[cfg(feature = "pty")]
pub(crate) fn mono_cell() -> (f32, f32) {
    let style = TextStyle {
        font_family: "mono".to_string(),
        ..TextStyle::default()
    };
    (advance(&style).unwrap_or_else(col), row())
}

const fn downstream(byte_index: usize) -> HitPosition {
    HitPosition {
        byte_index,
        affinity: TextAffinity::Downstream,
    }
}

fn dimension(value: Length) -> Length {
    match value {
        Length::Rows(_) | Length::Cols(_) | Length::Em(_) => {
            finite_dimension(value.text_px().unwrap_or(0.0))
        }
        Length::Px(px) => finite_dimension(px),
        Length::Percent(pct) if !pct.is_finite() => Length::Auto,
        other => other,
    }
}

const fn finite_dimension(px: f32) -> Length {
    if px.is_finite() {
        Length::Px(px)
    } else {
        Length::Auto
    }
}

const fn finite_edge(value: Length) -> Length {
    finite_edge_or(value, Length::Px(0.0))
}

const fn finite_edge_or(value: Length, fallback: Length) -> Length {
    let finite = match value {
        Length::Px(px)
        | Length::Percent(px)
        | Length::Rows(px)
        | Length::Cols(px)
        | Length::Em(px) => px.is_finite(),
        Length::Auto | Length::Fill => true,
    };
    if finite { value } else { fallback }
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
    style.size = style.size.map(dimension);
    style.min_size = style.min_size.map(dimension);
    style.max_size = style.max_size.map(dimension);
    style.gap = style.gap.map(finite_edge);
    style.padding = style.padding.map(finite_edge);
    style.border = style.border.map(finite_edge);
    style.margin = style
        .margin
        .map(|value| finite_edge_or(value, Length::Px(0.0)));
    style.inset = style.inset.map(|value| finite_edge_or(value, Length::Auto));
    style.aspect_ratio = style
        .aspect_ratio
        .filter(|ratio| ratio.is_finite() && *ratio > 0.0);
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
