//! Parley-backed text engine implementation.

use std::cell::RefCell;

use std::borrow::Cow;

use parley::style::{FontFamily, FontWeight, LineHeight};
use parley::{FontContext, LayoutContext, StyleProperty};
use sigil_hal::Color;
use sigil_hal::TextEngine;
use sigil_style::{StyledTextEngine, TextLineHeight, TextStyle};

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

    /// Lay out a paragraph with explicit resolved typography.
    ///
    /// # Errors
    ///
    /// Returns [`TextError`] if paragraph layout fails.
    pub fn layout_paragraph_styled(
        &self,
        text: &str,
        max_width: Option<f32>,
        style: &TextStyle,
    ) -> Result<Paragraph, TextError> {
        if let Some(cached) = self.cache.borrow().get(text, max_width, style) {
            return Ok(cached.clone());
        }

        let mut font_cx = self.font_cx.borrow_mut();
        let mut layout_cx = self.layout_cx.borrow_mut();

        let scale = 1.0;
        let quantize = true;
        let mut builder = layout_cx.ranged_builder(&mut font_cx, text, scale, quantize);

        let family = style.font_family.as_str();
        builder.push_default(StyleProperty::FontFamily(FontFamily::Source(
            Cow::Borrowed(family),
        )));
        builder.push_default(StyleProperty::FontSize(style.font_size.max(1.0)));
        builder.push_default(StyleProperty::FontWeight(FontWeight::new(f32::from(
            style.font_weight,
        ))));
        builder.push_default(StyleProperty::LineHeight(to_parley_line_height(
            style.line_height,
        )));
        builder.push_default(StyleProperty::Brush(Color::WHITE));

        let mut layout = builder.build(text);
        layout.break_all_lines(max_width);

        let para = Paragraph::new(layout);
        self.cache
            .borrow_mut()
            .insert(text, max_width, style, para.clone());
        Ok(para)
    }
}

impl TextEngine for ParleyTextEngine {
    type Error = TextError;
    type Paragraph = Paragraph;

    fn layout_paragraph(&self, text: &str, max_width: Option<f32>) -> Result<Paragraph, TextError> {
        let style = self.config.text_style();
        self.layout_paragraph_styled(text, max_width, &style)
    }

    fn clear_cache(&self) {
        self.cache.borrow_mut().clear();
    }
}

impl StyledTextEngine for ParleyTextEngine {
    fn layout_paragraph_styled(
        &self,
        text: &str,
        max_width: Option<f32>,
        style: &TextStyle,
    ) -> Result<Paragraph, TextError> {
        Self::layout_paragraph_styled(self, text, max_width, style)
    }
}

const fn to_parley_line_height(line_height: TextLineHeight) -> LineHeight {
    match line_height {
        TextLineHeight::FontSizeRelative(value) => LineHeight::FontSizeRelative(value.max(0.1)),
        TextLineHeight::Absolute(value) => LineHeight::Absolute(value.max(1.0)),
    }
}
