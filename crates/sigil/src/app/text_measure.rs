use std::rc::Rc;

use crate::geometry::{Point, Rect};
use crate::hal::{HitPosition, TextAffinity};
use crate::layout::text_adapter::TextEngineMeasure;
use crate::layout::{CaretGeometry, TextMeasure};
use crate::style::{StyledTextEngine as _, TextStyle};
use crate::text::OwnedTextEngine;

use super::cell_text::floor_boundary;

/// `TextMeasure` implementation backed by the thread text engine.
pub struct DefaultTextMeasure {
    inner: TextEngineMeasure<OwnedTextEngine>,
    engine: Rc<OwnedTextEngine>,
}

impl Default for DefaultTextMeasure {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultTextMeasure {
    /// Creates a default text measure.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: TextEngineMeasure::new(OwnedTextEngine::thread()),
            engine: OwnedTextEngine::thread(),
        }
    }
}

impl TextMeasure for DefaultTextMeasure {
    fn measure(
        &self,
        content: &crate::kernel::TextContent,
        space: crate::geometry::Size,
    ) -> crate::geometry::LogicalSize {
        self.inner.measure(content, space)
    }

    fn measure_styled(
        &self,
        content: &crate::kernel::TextContent,
        style: &TextStyle,
        space: crate::geometry::Size,
    ) -> crate::geometry::LogicalSize {
        self.inner.measure_styled(content, style, space)
    }

    fn begin_frame(&self) {
        self.inner.begin_frame();
    }

    fn font_generation(&self) -> u64 {
        self.inner.font_generation()
    }

    fn hit(&self, text: &str, style: &TextStyle, width: Option<f32>, point: Point) -> HitPosition {
        self.engine
            .layout_paragraph_styled(text, width, style)
            .map_or(
                HitPosition {
                    byte_index: 0,
                    affinity: TextAffinity::Downstream,
                },
                |para| para.hit_test(point.x, point.y),
            )
    }

    fn caret(
        &self,
        text: &str,
        style: &TextStyle,
        width: Option<f32>,
        mut position: HitPosition,
    ) -> CaretGeometry {
        position.byte_index = floor_boundary(text, position.byte_index);
        self.engine
            .layout_paragraph_styled(text, width, style)
            .map_or_else(
                |_| CaretGeometry::default(),
                |para| {
                    let rect = para.caret_rect(position, 1.0);
                    CaretGeometry {
                        x: rect.origin.x,
                        top: rect.origin.y,
                        height: rect.height,
                    }
                },
            )
    }

    fn selection_rects(
        &self,
        text: &str,
        style: &TextStyle,
        width: Option<f32>,
        (mut anchor, mut focus): (HitPosition, HitPosition),
    ) -> Vec<Rect> {
        anchor.byte_index = floor_boundary(text, anchor.byte_index);
        focus.byte_index = floor_boundary(text, focus.byte_index);
        self.engine
            .layout_paragraph_styled(text, width, style)
            .map_or_else(|_| Vec::new(), |para| para.selection_rects(anchor, focus))
    }
}
