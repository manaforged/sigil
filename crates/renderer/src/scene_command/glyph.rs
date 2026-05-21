//! Glyph-run command payloads.

use core::fmt;

use hal::{Rect, RenderedGlyph};
use kernel::NodeId;
use style::{Rgba8, TextStyle};
use text::Paragraph;

/// Text run payload carried by a glyph-run command.
#[derive(Clone)]
pub struct GlyphRun {
    /// Source node, when emitted from a core tree.
    pub node: Option<NodeId>,
    /// Layout rectangle for the text run.
    pub rect: Rect,
    /// Text content used by the interim text path.
    pub text: String,
    /// Resolved text colour.
    pub color: Rgba8,
    /// Resolved typography.
    pub style: TextStyle,
    /// Opacity encoded as `value * 10000`.
    pub opacity: u16,
    positioned_glyphs: Vec<RenderedGlyph>,
    paragraph: Option<Paragraph>,
}

impl GlyphRun {
    /// Create a glyph-run payload.
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
            text: text.into(),
            color,
            style,
            opacity,
            positioned_glyphs: Vec::new(),
            paragraph: None,
        }
    }

    /// Return `true` when this run carries shaped glyph positions.
    #[must_use]
    pub const fn has_positioned_glyphs(&self) -> bool {
        !self.positioned_glyphs.is_empty()
    }

    /// Borrow shaped glyph positions.
    #[must_use]
    pub fn positioned_glyphs(&self) -> &[RenderedGlyph] {
        &self.positioned_glyphs
    }

    #[allow(dead_code)]
    pub(crate) const fn paragraph(&self) -> Option<&Paragraph> {
        self.paragraph.as_ref()
    }

    #[allow(dead_code)]
    pub(crate) fn with_paragraph(mut self, paragraph: Paragraph) -> Self {
        self.positioned_glyphs = paragraph.rendered_glyphs();
        self.paragraph = Some(paragraph);
        self
    }

    pub(crate) const fn with_node(mut self, node: NodeId) -> Self {
        self.node = Some(node);
        self
    }
}

impl fmt::Debug for GlyphRun {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GlyphRun")
            .field("node", &self.node)
            .field("rect", &self.rect)
            .field("text", &self.text)
            .field("color", &self.color)
            .field("style", &self.style)
            .field("opacity", &self.opacity)
            .field("positioned_glyphs", &self.positioned_glyphs)
            .field("has_paragraph", &self.paragraph.is_some())
            .finish()
    }
}

impl PartialEq for GlyphRun {
    fn eq(&self, other: &Self) -> bool {
        self.rect == other.rect
            && self.node == other.node
            && self.text == other.text
            && self.color == other.color
            && self.style == other.style
            && self.opacity == other.opacity
            && self.positioned_glyphs == other.positioned_glyphs
            && self.paragraph.is_some() == other.paragraph.is_some()
    }
}

/// Glyph-run draw payload.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawGlyphRun {
    /// Glyph-run payload.
    pub run: GlyphRun,
}

impl DrawGlyphRun {
    /// Create a glyph-run draw payload.
    #[must_use]
    pub const fn new(run: GlyphRun) -> Self {
        Self { run }
    }
}
