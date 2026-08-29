use core::fmt;
use std::ops::Deref;
use std::rc::Rc;

use sigil_hal::{Rect, RenderedGlyph};
use sigil_kernel::NodeId;
use sigil_style::{Rgba8, TextStyle};
use sigil_text::Paragraph;

#[derive(Clone)]
pub struct SharedGlyph {
    pub text: String,
    pub color: Rgba8,
    pub style: TextStyle,
    pub opacity: u16,
    positioned_glyphs: Vec<RenderedGlyph>,
    paragraph: Option<Paragraph>,
}

#[derive(Clone)]
pub struct GlyphRun {
    pub node: Option<NodeId>,
    pub rect: Rect,
    shared: Rc<SharedGlyph>,
}

impl GlyphRun {
    #[must_use]
    pub fn new(
        rect: Rect,
        text: impl Into<String>,
        color: Rgba8,
        style: TextStyle,
        opacity: u16,
    ) -> Self {
        Self {
            node: None,
            rect,
            shared: Rc::new(SharedGlyph {
                text: text.into(),
                color,
                style,
                opacity,
                positioned_glyphs: Vec::new(),
                paragraph: None,
            }),
        }
    }

    #[must_use]
    pub fn has_positioned_glyphs(&self) -> bool {
        !self.shared.positioned_glyphs.is_empty()
    }

    #[must_use]
    pub fn positioned_glyphs(&self) -> &[RenderedGlyph] {
        &self.shared.positioned_glyphs
    }

    #[allow(dead_code, reason = "intentional, reviewed trade-off")]
    pub(crate) fn paragraph(&self) -> Option<&Paragraph> {
        self.shared.paragraph.as_ref()
    }

    #[allow(dead_code, reason = "intentional, reviewed trade-off")]
    pub(crate) fn with_paragraph(mut self, paragraph: Paragraph) -> Self {
        let shared = Rc::make_mut(&mut self.shared);
        shared.positioned_glyphs = paragraph.rendered_glyphs();
        shared.paragraph = Some(paragraph);
        self
    }

    pub(crate) const fn with_node(mut self, node: NodeId) -> Self {
        self.node = Some(node);
        self
    }
}

impl Deref for GlyphRun {
    type Target = SharedGlyph;

    fn deref(&self) -> &SharedGlyph {
        &self.shared
    }
}

impl fmt::Debug for GlyphRun {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GlyphRun")
            .field("node", &self.node)
            .field("rect", &self.rect)
            .field("text", &self.shared.text)
            .field("color", &self.shared.color)
            .field("style", &self.shared.style)
            .field("opacity", &self.shared.opacity)
            .field("positioned_glyphs", &self.shared.positioned_glyphs)
            .field("has_paragraph", &self.shared.paragraph.is_some())
            .finish()
    }
}

impl PartialEq for GlyphRun {
    fn eq(&self, other: &Self) -> bool {
        self.rect == other.rect
            && self.node == other.node
            && self.shared.text == other.shared.text
            && self.shared.color == other.shared.color
            && self.shared.style == other.shared.style
            && self.shared.opacity == other.shared.opacity
            && self.shared.positioned_glyphs == other.shared.positioned_glyphs
            && self.shared.paragraph.is_some() == other.shared.paragraph.is_some()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DrawGlyphRun {
    pub run: GlyphRun,
}

impl DrawGlyphRun {
    #[must_use]
    pub const fn new(run: GlyphRun) -> Self {
        Self { run }
    }
}

#[doc(hidden)]
#[must_use]
pub fn shape_glyph_run(
    rect: Rect,
    text: String,
    color: Rgba8,
    style: TextStyle,
    opacity: u16,
) -> GlyphRun {
    shape_glyph_run_with(rect, text, color, style, opacity, false)
}

#[allow(
    clippy::too_many_arguments,
    reason = "the argument list mirrors the upstream API contract"
)]
#[must_use]
pub(crate) fn shape_glyph_run_with(
    rect: Rect,
    text: String,
    color: Rgba8,
    style: TextStyle,
    opacity: u16,
    ellipsize: bool,
) -> GlyphRun {
    let max_width = rect.width.max(1.0);
    let engine = sigil_text::OwnedTextEngine::thread();
    let paragraph = if ellipsize {
        engine.layout_paragraph_ellipsized(&text, max_width, &style)
    } else {
        engine.layout_paragraph_styled(&text, Some(max_width), &style)
    };
    let run = GlyphRun::new(rect, text, color, style, opacity);
    match paragraph {
        Ok(paragraph) => run.with_paragraph(paragraph),
        Err(err) => {
            tracing::warn!("text layout failed, run paints no glyphs: {err}");
            run
        }
    }
}
