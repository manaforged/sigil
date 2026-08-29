use std::rc::Rc;

use sigil_hal::{HitPosition, TextAffinity};
use sigil_layout::{CaretGeometry, TextGeometry};
use sigil_style::TextStyle;
use sigil_text::OwnedTextEngine;

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
    fn hit(&self, text: &str, style: &TextStyle, x: f32) -> usize {
        self.engine
            .layout_paragraph_styled(text, None, style)
            .map_or(0, |para| para.hit_test(x, 0.0).byte_index)
    }

    fn caret(&self, text: &str, style: &TextStyle, byte: usize) -> CaretGeometry {
        let byte = clamp_to_boundary(text, byte);
        self.engine
            .layout_paragraph_styled(text, None, style)
            .map_or_else(
                |_| CaretGeometry::default(),
                |para| {
                    let pos = HitPosition {
                        byte_index: byte,
                        affinity: TextAffinity::Downstream,
                    };
                    let rect = para.caret_rect(pos, 1.0);
                    CaretGeometry {
                        x: rect.origin.x,
                        top: rect.origin.y,
                        height: rect.height,
                    }
                },
            )
    }
}

fn clamp_to_boundary(text: &str, byte: usize) -> usize {
    let mut byte = byte.min(text.len());
    while byte > 0 && !text.is_char_boundary(byte) {
        byte -= 1;
    }
    byte
}
