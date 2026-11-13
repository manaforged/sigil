use crate::geometry::{Constraint, Point, Rect, Size};
use crate::hal::{HitPosition, TextAffinity};
use crate::kernel::TextContent;
use crate::layout::CaretGeometry;
use crate::style::TextStyle;

/// Map between points and text positions for hit testing, carets, and selection.
#[doc(hidden)]
pub trait TextMeasure {
    /// Returns the size of `content` in `space`.
    fn measure(&self, content: &TextContent, space: Constraint) -> Size;

    /// Returns the size of `content` with `style` in `space`.
    fn measure_styled(&self, content: &TextContent, style: &TextStyle, space: Constraint) -> Size {
        let _ = style;
        self.measure(content, space)
    }

    /// Runs before each layout pass.
    fn begin_frame(&self) {}

    /// A number that is part of the key of cached text sizes; the default is 0.
    fn font_generation(&self) -> u64 {
        0
    }

    /// Returns the text position at `point`.
    fn hit(&self, text: &str, style: &TextStyle, width: Option<f32>, point: Point) -> HitPosition {
        let _ = (text, style, width, point);
        HitPosition {
            byte_index: 0,
            affinity: TextAffinity::Downstream,
        }
    }

    /// The caret geometry at `position` in `text`; the default is an empty geometry.
    fn caret(
        &self,
        text: &str,
        style: &TextStyle,
        width: Option<f32>,
        position: HitPosition,
    ) -> CaretGeometry {
        let _ = (text, style, width, position);
        CaretGeometry::default()
    }

    /// The rectangles that cover `selection` in `text`; the default is none.
    fn selection_rects(
        &self,
        text: &str,
        style: &TextStyle,
        width: Option<f32>,
        selection: (HitPosition, HitPosition),
    ) -> Vec<Rect> {
        let _ = (text, style, width, selection);
        Vec::new()
    }
}

impl<T: TextMeasure + ?Sized> TextMeasure for std::rc::Rc<T> {
    fn measure(&self, content: &TextContent, space: Constraint) -> Size {
        (**self).measure(content, space)
    }

    fn measure_styled(&self, content: &TextContent, style: &TextStyle, space: Constraint) -> Size {
        (**self).measure_styled(content, style, space)
    }

    fn begin_frame(&self) {
        (**self).begin_frame();
    }

    fn font_generation(&self) -> u64 {
        (**self).font_generation()
    }

    fn hit(&self, text: &str, style: &TextStyle, width: Option<f32>, point: Point) -> HitPosition {
        (**self).hit(text, style, width, point)
    }

    fn caret(
        &self,
        text: &str,
        style: &TextStyle,
        width: Option<f32>,
        position: HitPosition,
    ) -> CaretGeometry {
        (**self).caret(text, style, width, position)
    }

    fn selection_rects(
        &self,
        text: &str,
        style: &TextStyle,
        width: Option<f32>,
        selection: (HitPosition, HitPosition),
    ) -> Vec<Rect> {
        (**self).selection_rects(text, style, width, selection)
    }
}

#[cfg(feature = "kittest")]
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FakeMeasure {
    width: f32,
    height: f32,
    per_char: bool,
}

#[cfg(feature = "kittest")]
impl FakeMeasure {
    const CARET_ADVANCE: f32 = 10.0;
    const CARET_HEIGHT: f32 = 16.0;

    #[must_use]
    pub const fn fixed(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            per_char: false,
        }
    }

    #[must_use]
    pub const fn chars(char_width: f32, line_height: f32) -> Self {
        Self {
            width: char_width,
            height: line_height,
            per_char: true,
        }
    }
}

#[cfg(feature = "kittest")]
impl TextMeasure for FakeMeasure {
    #[allow(
        clippy::cast_precision_loss,
        reason = "test text lengths are far below f32 precision limits"
    )]
    fn measure(&self, content: &TextContent, _space: Constraint) -> Size {
        if !self.per_char {
            return Size::new(self.width, self.height);
        }
        let text = content.resolve();
        let columns = text.lines().map(|line| line.chars().count()).max();
        let rows = text.lines().count().max(1);
        Size::new(
            columns.unwrap_or(0) as f32 * self.width,
            rows as f32 * self.height,
        )
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the point is clamped to zero and the text length"
    )]
    fn hit(&self, text: &str, _: &TextStyle, _: Option<f32>, point: Point) -> HitPosition {
        HitPosition {
            byte_index: ((point.x / Self::CARET_ADVANCE).round().max(0.0) as usize).min(text.len()),
            affinity: TextAffinity::Downstream,
        }
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "test text lengths are far below f32 precision limits"
    )]
    fn caret(&self, text: &str, _: &TextStyle, _: Option<f32>, at: HitPosition) -> CaretGeometry {
        let byte = at.byte_index.min(text.len());
        let prefix = text.as_bytes().get(..byte).unwrap_or_default();
        let line = prefix.split(|item| *item == b'\n').count() - 1;
        let start = prefix
            .iter()
            .rposition(|item| *item == b'\n')
            .map_or(0, |newline| newline + 1);
        CaretGeometry {
            x: (byte - start) as f32 * Self::CARET_ADVANCE,
            top: line as f32 * Self::CARET_HEIGHT,
            height: Self::CARET_HEIGHT,
        }
    }
}
