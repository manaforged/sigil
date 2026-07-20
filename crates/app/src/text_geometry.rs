//! Default [`TextGeometry`] provider installed by `sigil::launch`.
//!
//! Backs caret/selection hit-testing for text widgets over an owned text
//! engine configured identically to [`DefaultTextMeasure`](crate::DefaultTextMeasure)
//! — same [`TextConfig::default`] + process default fonts — so the geometry a
//! widget reads matches the glyphs the renderer paints (shaping is
//! deterministic for a given text + style + font set).

use sigil_hal::{HitPosition, TextAffinity};
use sigil_layout::{CaretGeometry, TextGeometry};
use sigil_style::TextStyle;
use sigil_text::{OwnedTextEngine, TextConfig};

/// Owned-text-engine-backed geometry provider for single-line text widgets.
pub struct EngineGeometry {
    engine: OwnedTextEngine,
}

impl Default for EngineGeometry {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineGeometry {
    /// Create the default geometry provider.
    #[must_use]
    pub fn new() -> Self {
        Self {
            engine: OwnedTextEngine::new(TextConfig::default()),
        }
    }
}

impl TextGeometry for EngineGeometry {
    fn hit(&self, text: &str, style: &TextStyle, x: f32) -> usize {
        // `None` width = single line, no wrap — matches a text field's track.
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

/// Clamp `byte` to the nearest UTF-8 boundary at or below `text.len()`.
fn clamp_to_boundary(text: &str, byte: usize) -> usize {
    let mut byte = byte.min(text.len());
    while byte > 0 && !text.is_char_boundary(byte) {
        byte -= 1;
    }
    byte
}
