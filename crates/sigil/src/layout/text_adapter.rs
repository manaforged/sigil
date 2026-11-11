use std::rc::Rc;

use crate::geometry::{Dim, LogicalSize, Point, Rect, Size};
use crate::hal::{HitPosition, TextAffinity};
use crate::kernel::TextContent;
use crate::style::TextStyle;
use crate::text::OwnedTextEngine;

use crate::layout::{CaretGeometry, TextMeasure};

#[derive(Debug)]
pub struct TextEngineMeasure {
    engine: Rc<OwnedTextEngine>,
}

impl TextEngineMeasure {
    #[must_use]
    pub const fn new(engine: Rc<OwnedTextEngine>) -> Self {
        Self { engine }
    }
}

impl Default for TextEngineMeasure {
    fn default() -> Self {
        Self::new(OwnedTextEngine::thread())
    }
}

impl TextMeasure for TextEngineMeasure {
    fn measure(&self, source: &TextContent, space: Size) -> LogicalSize {
        self.measure_styled(source, &TextStyle::default(), space)
    }

    fn measure_styled(&self, source: &TextContent, style: &TextStyle, space: Size) -> LogicalSize {
        crate::profile::count(crate::profile::Counter::TextMeasure);
        let max_width = match space.width {
            Dim::Fixed(w) => Some(w),
            Dim::MinContent | Dim::MaxContent => None,
        };
        let result = source.with_str(|text| self.engine.layout_paragraph_styled(text, max_width, style));
        let para = match result {
            Ok(para) => para,
            Err(err) => {
                tracing::warn!(target: "sigil::text", error = %err, "text measure failed; reporting zero size");
                return LogicalSize::default();
            }
        };
        let width = match space.width {
            Dim::Fixed(_) | Dim::MaxContent => para.width(),
            Dim::MinContent => para.min_content_width(),
        };
        LogicalSize::new(width, para.height())
    }

    fn begin_frame(&self) {}

    fn font_generation(&self) -> u64 {
        self.engine.font_generation()
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
        position.byte_index = text.floor_char_boundary(position.byte_index);
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
        anchor.byte_index = text.floor_char_boundary(anchor.byte_index);
        focus.byte_index = text.floor_char_boundary(focus.byte_index);
        self.engine
            .layout_paragraph_styled(text, width, style)
            .map_or_else(|_| Vec::new(), |para| para.selection_rects(anchor, focus))
    }
}
