use core::fmt;
use std::ops::Deref;
use std::sync::Arc;

use crate::geometry::Rect;
use crate::hal::RenderedGlyph;
use crate::kernel::NodeId;
use crate::style::{Color, TextStyle, Tone};
#[cfg(feature = "scene")]
use crate::text::Paragraph;

kittest_pub! {
    /// The direction of a run of text.
    #[doc(hidden)]
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub enum TextFlow {
        /// The line text flow.
        #[default]
        Line,
        /// The wrap text flow.
        Wrap,
        /// The ellipsis text flow.
        Ellipsis,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RunInk {
    pub(crate) color: Option<Color>,
    pub(crate) background: Option<Color>,
}

/// The text, style, and shaped glyphs that a `GlyphRun` shares between clones.
#[derive(Clone)]
pub struct SharedGlyph {
    /// The text.
    pub text: String,
    /// The color.
    pub color: Color,
    /// The text style.
    pub style: TextStyle,
    /// The opacity.
    pub opacity: u16,
    #[cfg(feature = "kittest")]
    #[doc(hidden)]
    pub flow: TextFlow,
    #[cfg(not(feature = "kittest"))]
    pub(crate) flow: TextFlow,
    /// The tone, if any.
    pub tone: Option<Tone>,
    inks: Vec<RunInk>,
    positioned_glyphs: Vec<RenderedGlyph>,
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
            .field("positioned_glyphs", &self.positioned_glyphs)
            .finish_non_exhaustive()
    }
}

/// Shaped glyphs with one style and origin.
#[derive(Clone)]
pub struct GlyphRun {
    /// The node, if any.
    pub node: Option<NodeId>,
    /// The rect.
    pub rect: Rect,
    shared: Arc<SharedGlyph>,
}

impl GlyphRun {
    /// Creates a glyph run.
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
                positioned_glyphs: Vec::new(),
                #[cfg(feature = "scene")]
                paragraph: None,
                fill: None,
            }),
        }
    }

    /// Returns the number of positioned glyphs in this run.
    #[must_use]
    pub fn glyph_count(&self) -> usize {
        self.shared.positioned_glyphs.len()
    }

    #[cfg(feature = "scene")]
    #[must_use]
    pub(crate) fn positioned_glyphs(&self) -> &[RenderedGlyph] {
        &self.shared.positioned_glyphs
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
        shared.positioned_glyphs = paragraph.rendered_glyphs();
        shared.paragraph = Some(paragraph);
        self
    }

    #[cfg(feature = "scene")]
    pub(crate) fn with_glyphs(mut self, color: Color, glyphs: Vec<RenderedGlyph>) -> Self {
        let shared = Arc::make_mut(&mut self.shared);
        shared.color = color;
        shared.positioned_glyphs = glyphs;
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
const fn same_paragraph(left: &SharedGlyph, right: &SharedGlyph) -> bool {
    left.paragraph.is_some() == right.paragraph.is_some()
}

#[cfg(not(feature = "scene"))]
const fn same_paragraph(_: &SharedGlyph, _: &SharedGlyph) -> bool {
    true
}

impl Deref for GlyphRun {
    type Target = SharedGlyph;

    fn deref(&self) -> &SharedGlyph {
        &self.shared
    }
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
            .field("positioned_glyphs", &self.shared.positioned_glyphs);
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
        && this.positioned_glyphs == other.positioned_glyphs
        && this.fill == other.fill
        && same_paragraph(this, other)
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
    let paragraph = match run.flow {
        TextFlow::Ellipsis => engine.layout_paragraph_ellipsized(&run.text, max_width, &run.style),
        TextFlow::Line => engine.layout_paragraph_styled(&run.text, None, &run.style),
        TextFlow::Wrap => engine.layout_paragraph_styled(&run.text, Some(max_width), &run.style),
    };
    match paragraph {
        Ok(paragraph) => run.with_paragraph(paragraph),
        Err(err) => {
            tracing::warn!("text layout failed, run paints no glyphs: {err}");
            run
        }
    }
}
