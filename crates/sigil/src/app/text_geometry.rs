use std::rc::Rc;

use crate::hal::{HitPosition, Point, Rect, TextAffinity};
use crate::layout::{CaretGeometry, TextGeometry};
use crate::style::TextStyle;
use crate::text::OwnedTextEngine;

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
        position.byte_index = clamp_to_boundary(text, position.byte_index);
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
        (anchor, focus): (HitPosition, HitPosition),
    ) -> Vec<Rect> {
        let anchor = HitPosition {
            byte_index: clamp_to_boundary(text, anchor.byte_index),
            ..anchor
        };
        let focus = HitPosition {
            byte_index: clamp_to_boundary(text, focus.byte_index),
            ..focus
        };
        self.engine
            .layout_paragraph_styled(text, width, style)
            .map_or_else(|_| Vec::new(), |para| para.selection_rects(anchor, focus))
    }
}

fn clamp_to_boundary(text: &str, byte: usize) -> usize {
    let mut byte = byte.min(text.len());
    while byte > 0 && !text.is_char_boundary(byte) {
        byte -= 1;
    }
    byte
}
