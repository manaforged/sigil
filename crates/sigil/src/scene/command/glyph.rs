use core::fmt;
use std::sync::Arc;

use crate::geometry::Rect;
use crate::hal::RenderedGlyph;
use crate::kernel::NodeId;
use crate::style::{Color, TextStyle, Tone};
#[cfg(feature = "scene")]
use crate::text::Paragraph;

kittest_pub! {
    /// How a run of text fits the width of its rectangle.
    #[doc(hidden)]
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub enum TextFlow {
        /// One line that is laid out without a width limit.
        #[default]
        Line,
        /// Lines break at the width of the rectangle.
        Wrap,
        /// One line, cut with an ellipsis at the width of the rectangle.
        Ellipsis,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RunInk {
    pub(crate) color: Option<Color>,
    pub(crate) background: Option<Color>,
}

#[derive(Clone)]
pub(crate) struct SharedGlyph {
    pub(crate) text: String,
    pub(crate) color: Color,
    pub(crate) style: TextStyle,
    pub(crate) opacity: u16,
    pub(crate) flow: TextFlow,
    pub(crate) tone: Option<Tone>,
    inks: Vec<RunInk>,
    positioned_glyphs: Option<Vec<RenderedGlyph>>,
    #[cfg(feature = "scene")]
    paragraph: Option<Paragraph>,
    fill: Option<super::DrawPath>,
}

impl core::fmt::Debug for SharedGlyph {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SharedGlyph")
            .field("text", &self.text)
            .field("color", &self.color)
            .field("style", &self.style)
            .field("opacity", &self.opacity)
            .field("flow", &self.flow)
            .field("tone", &self.tone)
            .field("positioned_glyphs", &self.glyphs_slice())
            .finish_non_exhaustive()
    }
}

impl SharedGlyph {
    #[allow(
        clippy::missing_const_for_fn,
        reason = "scene builds read Paragraph through Arc"
    )]
    fn glyph_count(&self) -> usize {
        if let Some(glyphs) = &self.positioned_glyphs {
            return glyphs.len();
        }
        #[cfg(feature = "scene")]
        if let Some(paragraph) = &self.paragraph {
            return paragraph.glyph_count();
        }
        0
    }

    fn glyphs_slice(&self) -> &[RenderedGlyph] {
        if let Some(glyphs) = &self.positioned_glyphs {
            return glyphs;
        }
        #[cfg(feature = "scene")]
        if let Some(paragraph) = &self.paragraph {
            return paragraph.rendered_glyphs_slice();
        }
        &[]
    }
}

/// Shaped glyphs with one style and origin.
#[derive(Clone)]
#[non_exhaustive]
pub struct GlyphRun {
    /// The node this text belongs to, or `None` when no node owns it.
    pub node: Option<NodeId>,
    /// The box the text is laid out in, in logical pixels; glyph positions are relative to its origin.
    pub rect: Rect,
    shared: Arc<SharedGlyph>,
}

impl GlyphRun {
    /// Creates a one-line text run in `rect` with no node and no positioned glyphs.
    #[must_use]
    pub fn new(
        rect: Rect,
        text: impl Into<String>,
        color: Color,
        style: TextStyle,
        opacity: u16,
    ) -> Self {
        Self {
            node: None,
            rect,
            shared: Arc::new(SharedGlyph {
                text: text.into(),
                color,
                style,
                opacity,
                flow: TextFlow::Line,
                tone: None,
                inks: Vec::new(),
                positioned_glyphs: None,
                #[cfg(feature = "scene")]
                paragraph: None,
                fill: None,
            }),
        }
    }

    /// The string that this run draws, before shaping.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.shared.text
    }

    /// The color the glyphs are painted with.
    #[must_use]
    pub fn color(&self) -> Color {
        self.shared.color
    }

    /// The font, size, weight, and decoration the run is shaped with.
    #[must_use]
    pub fn style(&self) -> &TextStyle {
        &self.shared.style
    }

    /// The opacity in basis points; 10000 is opaque.
    #[must_use]
    pub fn opacity(&self) -> u16 {
        self.shared.opacity
    }

    /// The theme tone the terminal host inks the text with when the color is fully transparent.
    #[must_use]
    pub fn tone(&self) -> Option<Tone> {
        self.shared.tone
    }

    #[cfg(vector_paint)]
    pub(crate) fn flow(&self) -> TextFlow {
        self.shared.flow
    }

    /// The number of positioned glyphs in this run.
    #[must_use]
    pub fn glyph_count(&self) -> usize {
        self.shared.glyph_count()
    }

    #[cfg(feature = "scene")]
    #[must_use]
    pub(crate) fn positioned_glyphs(&self) -> &[RenderedGlyph] {
        self.shared.glyphs_slice()
    }

    #[cfg(scene_host)]
    pub(crate) fn ink_rect(&self) -> Option<Rect> {
        let Some(paragraph) = self.paragraph() else {
            return Some(self.rect);
        };
        let origin = self.rect.origin;
        let ink = paragraph.ink_rect()?;
        Some(Rect::new(
            crate::geometry::Point::new(origin.x + ink.origin.x, origin.y + ink.origin.y),
            ink.width,
            ink.height,
        ))
    }

    #[cfg(not(feature = "scene"))]
    #[cfg(feature = "terminal")]
    #[expect(
        clippy::unnecessary_wraps,
        reason = "one signature with the shaped version, which can find no ink"
    )]
    pub(crate) const fn ink_rect(&self) -> Option<Rect> {
        Some(self.rect)
    }

    #[cfg(feature = "scene")]
    pub(crate) fn paragraph(&self) -> Option<&Paragraph> {
        self.shared.paragraph.as_ref()
    }

    #[cfg(feature = "scene")]
    pub(crate) fn with_paragraph(mut self, paragraph: Paragraph) -> Self {
        let shared = Arc::make_mut(&mut self.shared);
        shared.positioned_glyphs = None;
        shared.paragraph = Some(paragraph);
        self
    }

    #[cfg(feature = "scene")]
    pub(crate) fn with_glyphs(mut self, color: Color, glyphs: Vec<RenderedGlyph>) -> Self {
        let shared = Arc::make_mut(&mut self.shared);
        shared.color = color;
        shared.positioned_glyphs = Some(glyphs);
        self
    }

    pub(crate) fn with_inks(mut self, inks: Vec<RunInk>) -> Self {
        Arc::make_mut(&mut self.shared).inks = inks;
        self
    }

    #[cfg(vector_paint)]
    pub(crate) fn inks(&self) -> &[RunInk] {
        &self.shared.inks
    }

    pub(crate) fn with_cell(mut self, flow: TextFlow, tone: Option<Tone>) -> Self {
        let shared = Arc::make_mut(&mut self.shared);
        shared.flow = flow;
        shared.tone = tone;
        self
    }

    pub(crate) const fn with_node(mut self, node: NodeId) -> Self {
        self.node = Some(node);
        self
    }

    #[cfg(feature = "scene")]
    pub(crate) fn with_fill(mut self, fill: super::DrawPath) -> Self {
        Arc::make_mut(&mut self.shared).fill = Some(fill);
        self
    }

    #[cfg(feature = "scene")]
    pub(crate) fn fill(&self) -> Option<&super::DrawPath> {
        self.shared.fill.as_ref()
    }

    pub(crate) fn tinted(&self, color: Color) -> Self {
        let mut run = self.clone();
        run.node = None;
        let shared = Arc::make_mut(&mut run.shared);
        shared.color = color;
        shared.fill = None;
        for ink in &mut shared.inks {
            ink.color = ink.color.map(|_| color);
            ink.background = None;
        }
        run
    }

    pub(crate) fn shift(&mut self, dx: f32, dy: f32) {
        self.rect.origin.x += dx;
        self.rect.origin.y += dy;
        if self.shared.fill.is_some()
            && let Some(fill) = Arc::make_mut(&mut self.shared).fill.as_mut()
        {
            fill.shift(dx, dy);
        }
    }
}

#[cfg(feature = "scene")]
fn same_glyphs(left: &SharedGlyph, right: &SharedGlyph) -> bool {
    if left.paragraph.is_some() != right.paragraph.is_some() {
        return false;
    }
    if left.positioned_glyphs.is_none()
        && right.positioned_glyphs.is_none()
        && let (Some(lp), Some(rp)) = (&left.paragraph, &right.paragraph)
        && lp.ptr_eq(rp)
    {
        return true;
    }
    left.glyphs_slice() == right.glyphs_slice()
}

#[cfg(not(feature = "scene"))]
fn same_glyphs(left: &SharedGlyph, right: &SharedGlyph) -> bool {
    left.glyphs_slice() == right.glyphs_slice()
}

impl fmt::Debug for GlyphRun {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = f.debug_struct("GlyphRun");
        out.field("node", &self.node)
            .field("rect", &self.rect)
            .field("text", &self.shared.text)
            .field("color", &self.shared.color)
            .field("style", &self.shared.style)
            .field("opacity", &self.shared.opacity)
            .field("flow", &self.shared.flow)
            .field("tone", &self.shared.tone)
            .field("positioned_glyphs", &self.shared.glyphs_slice());
        #[cfg(feature = "scene")]
        out.field("has_paragraph", &self.shared.paragraph.is_some());
        out.finish()
    }
}

impl PartialEq for GlyphRun {
    fn eq(&self, other: &Self) -> bool {
        self.rect == other.rect
            && self.node == other.node
            && (Arc::ptr_eq(&self.shared, &other.shared)
                || same_shared(&self.shared, &other.shared))
    }
}

fn same_shared(this: &SharedGlyph, other: &SharedGlyph) -> bool {
    this.text == other.text
        && this.color == other.color
        && this.style == other.style
        && this.opacity == other.opacity
        && this.flow == other.flow
        && this.tone == other.tone
        && this.inks == other.inks
        && this.fill == other.fill
        && same_glyphs(this, other)
}

#[cfg(scene_host)]
kittest_pub! {
    /// A shaped `GlyphRun`, which `SceneCommand::GlyphRun` holds.
    #[must_use]
    pub fn shape_glyph_run(
        rect: Rect,
        text: String,
        color: Color,
        style: TextStyle,
        opacity: u16,
    ) -> GlyphRun {
        shape_run(GlyphRun::new(rect, text, color, style, opacity))
    }
}

#[must_use]
pub(crate) fn shape_run(run: GlyphRun) -> GlyphRun {
    if crate::style::cell_size().is_some() {
        return run;
    }
    shape_paragraph(run)
}

#[cfg(not(feature = "scene"))]
const fn shape_paragraph(run: GlyphRun) -> GlyphRun {
    run
}

#[cfg(feature = "scene")]
fn shape_paragraph(run: GlyphRun) -> GlyphRun {
    let max_width = run.rect.width.max(1.0);
    let engine = crate::text::OwnedTextEngine::thread();
    let paragraph = match run.flow() {
        TextFlow::Ellipsis => {
            engine.layout_paragraph_ellipsized(run.text(), max_width, run.style())
        }
        TextFlow::Line => engine.layout_paragraph_styled(run.text(), None, run.style()),
        TextFlow::Wrap => engine.layout_paragraph_styled(run.text(), Some(max_width), run.style()),
    };
    match paragraph {
        Ok(paragraph) => run.with_paragraph(paragraph),
        Err(err) => {
            tracing::warn!("text layout failed, run paints no glyphs: {err}");
            run
        }
    }
}
