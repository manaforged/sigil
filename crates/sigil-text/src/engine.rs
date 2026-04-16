//! Parley-backed text engine implementation.

use std::cell::RefCell;

use std::borrow::Cow;

use parley::style::{FontFamily, LineHeight};
use parley::{FontContext, LayoutContext, StyleProperty};
use sigil_hal::Color;
use sigil_hal::TextEngine;

use crate::cache::ParagraphCache;
use crate::config::TextConfig;
use crate::error::TextError;
use crate::paragraph::Paragraph;

/// Text engine backed by Parley v0.8.
///
/// Owns the font database, layout scratch space, and a per-frame
/// paragraph cache. All interior mutability is via `RefCell` —
/// single-threaded, zero-contention cost.
pub struct ParleyTextEngine {
    font_cx: RefCell<FontContext>,
    layout_cx: RefCell<LayoutContext<Color>>,
    config: TextConfig,
    cache: RefCell<ParagraphCache>,
}

impl ParleyTextEngine {
    /// Create a new text engine with the given default config.
    ///
    /// This enumerates system fonts, which can take 10–50 ms on
    /// systems with many installed fonts.
    #[must_use]
    pub fn new(config: TextConfig) -> Self {
        Self {
            font_cx: RefCell::new(FontContext::new()),
            layout_cx: RefCell::new(LayoutContext::new()),
            config,
            cache: RefCell::new(ParagraphCache::new()),
        }
    }

    /// Clear the paragraph cache. Call once per frame before layout.
    pub fn clear_cache(&self) {
        self.cache.borrow_mut().clear();
    }
}

impl TextEngine for ParleyTextEngine {
    type Error = TextError;
    type Paragraph = Paragraph;

    fn layout_paragraph(&self, text: &str, max_width: Option<f32>) -> Result<Paragraph, TextError> {
        if let Some(cached) = self.cache.borrow().get(text, max_width) {
            return Ok(cached.clone());
        }

        let mut font_cx = self.font_cx.borrow_mut();
        let mut layout_cx = self.layout_cx.borrow_mut();

        let scale = 1.0;
        let quantize = true;
        let mut builder = layout_cx.ranged_builder(&mut font_cx, text, scale, quantize);

        let family = self.config.font_family.as_str();
        builder.push_default(StyleProperty::FontFamily(FontFamily::Source(
            Cow::Borrowed(family),
        )));
        builder.push_default(StyleProperty::FontSize(self.config.font_size));
        builder.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
            self.config.line_height,
        )));
        builder.push_default(StyleProperty::Brush(Color::WHITE));

        let mut layout = builder.build(text);
        layout.break_all_lines(max_width);

        let para = Paragraph::new(layout);
        self.cache
            .borrow_mut()
            .insert(text, max_width, para.clone());
        Ok(para)
    }

    fn clear_cache(&self) {
        self.cache.borrow_mut().clear();
    }
}
