use std::rc::Rc;

use crate::geometry::{Point, Rect};
use crate::hal::{HitPosition, TextAffinity};
use crate::layout::{CaretGeometry, TextGeometry};
use crate::style::TextStyle;
use crate::text::OwnedTextEngine;

use super::cell_text::floor_boundary;

pub struct EngineGeometry {
    engine: Rc<OwnedTextEngine>,
}

impl Default for EngineGeometry {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineGeometry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            engine: OwnedTextEngine::thread(),
        }
    }
}

impl TextGeometry for EngineGeometry {
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
