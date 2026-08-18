//! Glyph-run command payloads.

use core::fmt;
use std::ops::Deref;
use std::rc::Rc;

use sigil_hal::{Rect, RenderedGlyph};
use sigil_kernel::NodeId;
use sigil_style::{Rgba8, TextStyle};
use sigil_text::Paragraph;

/// Shared glyph payload. Clone of [`GlyphRun`] is a refcount + rect.
#[derive(Clone)]
pub struct SharedGlyph {
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

/// Text run payload carried by a glyph-run command.
#[derive(Clone)]
pub struct GlyphRun {
    /// Source node, when emitted from a core tree.
    pub node: Option<NodeId>,
    /// Layout rectangle for the text run.
    pub rect: Rect,
    shared: Rc<SharedGlyph>,
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

    /// Return `true` when this run carries shaped glyph positions.
    #[must_use]
    pub fn has_positioned_glyphs(&self) -> bool {
        !self.shared.positioned_glyphs.is_empty()
    }

    /// Borrow shaped glyph positions.
    #[must_use]
    pub fn positioned_glyphs(&self) -> &[RenderedGlyph] {
        &self.shared.positioned_glyphs
    }

    #[allow(dead_code)]
    pub(crate) fn paragraph(&self) -> Option<&Paragraph> {
        self.shared.paragraph.as_ref()
    }

    #[allow(dead_code)]
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
