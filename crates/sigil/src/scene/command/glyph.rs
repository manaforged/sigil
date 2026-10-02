use core::fmt;
use std::ops::Deref;
use std::rc::Rc;

use crate::geometry::Rect;
use crate::hal::RenderedGlyph;
use crate::kernel::NodeId;
use crate::style::{Rgba8, TextStyle, Tone};
#[cfg(feature = "scene")]
use crate::text::Paragraph;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TextFlow {
    #[default]
    Line,
    Wrap,
    Ellipsis,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RunInk {
    pub(crate) color: Option<Rgba8>,
    pub(crate) background: Option<Rgba8>,
}

#[derive(Clone)]
pub struct SharedGlyph {
    pub text: String,
    pub color: Rgba8,
    pub style: TextStyle,
    pub opacity: u16,
    pub flow: TextFlow,
    pub tone: Option<Tone>,
    inks: Vec<RunInk>,
    positioned_glyphs: Vec<RenderedGlyph>,
    #[cfg(feature = "scene")]
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
                flow: TextFlow::Line,
                tone: None,
                inks: Vec::new(),
                positioned_glyphs: Vec::new(),
                #[cfg(feature = "scene")]
                paragraph: None,
            }),
        }
    }

    #[must_use]
    pub fn positioned_glyphs(&self) -> &[RenderedGlyph] {
        &self.shared.positioned_glyphs
    }

    #[cfg(feature = "scene")]
    pub(crate) fn paragraph(&self) -> Option<&Paragraph> {
        self.shared.paragraph.as_ref()
    }

    #[cfg(feature = "scene")]
    pub(crate) fn with_paragraph(mut self, paragraph: Paragraph) -> Self {
        let shared = Rc::make_mut(&mut self.shared);
        shared.positioned_glyphs = paragraph.rendered_glyphs();
        shared.paragraph = Some(paragraph);
        self
    }

    #[cfg(feature = "scene")]
    pub(crate) fn with_glyphs(mut self, color: Rgba8, glyphs: Vec<RenderedGlyph>) -> Self {
        let shared = Rc::make_mut(&mut self.shared);
        shared.color = color;
        shared.positioned_glyphs = glyphs;
        self
    }

    pub(crate) fn with_inks(mut self, inks: Vec<RunInk>) -> Self {
        Rc::make_mut(&mut self.shared).inks = inks;
        self
    }

    #[cfg(any(feature = "scene", feature = "terminal"))]
    pub(crate) fn inks(&self) -> &[RunInk] {
        &self.shared.inks
    }

    pub(crate) fn with_cell(mut self, flow: TextFlow, tone: Option<Tone>) -> Self {
        let shared = Rc::make_mut(&mut self.shared);
        shared.flow = flow;
        shared.tone = tone;
        self
    }

    pub(crate) const fn with_node(mut self, node: NodeId) -> Self {
        self.node = Some(node);
        self
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
            && (Rc::ptr_eq(&self.shared, &other.shared) || same_shared(&self.shared, &other.shared))
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
        && same_paragraph(this, other)
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

#[cfg(feature = "scene")]
#[must_use]
pub fn shape_glyph_run(
    rect: Rect,
    text: String,
    color: Rgba8,
    style: TextStyle,
    opacity: u16,
) -> GlyphRun {
    shape_run(GlyphRun::new(rect, text, color, style, opacity))
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
    let paragraph = if run.flow == TextFlow::Ellipsis {
        engine.layout_paragraph_ellipsized(&run.text, max_width, &run.style)
    } else {
        engine.layout_paragraph_styled(&run.text, Some(max_width), &run.style)
    };
    match paragraph {
        Ok(paragraph) => run.with_paragraph(paragraph),
        Err(err) => {
            tracing::warn!("text layout failed, run paints no glyphs: {err}");
            run
        }
    }
}
