//! Command-buffer scene representation.

#![allow(clippy::redundant_pub_crate)]

use core::fmt;

use sigil_core::NodeId;
use sigil_hal::Rect;
use sigil_hal::RenderedGlyph;
use sigil_style::{Px, Rgba8, TextStyle};
use sigil_text::Paragraph;

use crate::scene::{Scene, SceneItem, SceneRect, SceneText};

pub(crate) mod adapter;
pub(crate) mod walk;

/// Row-major 2D affine transform.
///
/// The two rows encode the transform from local coordinates to parent
/// coordinates as `x' = ax + cy + tx`, `y' = bx + dy + ty`.
pub type Transform = [[f32; 3]; 2];

/// A scene represented as ordered drawing and state commands.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommandScene {
    commands: Vec<SceneCommand>,
}

impl CommandScene {
    /// Create a command scene from raw commands.
    #[must_use]
    pub(crate) const fn new(commands: Vec<SceneCommand>) -> Self {
        Self { commands }
    }

    /// Borrow the ordered scene commands.
    #[must_use]
    pub fn commands(&self) -> &[SceneCommand] {
        &self.commands
    }

    /// Convert this command scene back to the legacy flat scene shape.
    #[deprecated(note = "migrate to CommandScene walk; removed in Phase 12")]
    #[must_use]
    pub fn to_flat(&self) -> Scene {
        let mut scene = Scene::default();
        for command in &self.commands {
            match command {
                SceneCommand::DrawRect(draw) => {
                    if let Some(node) = draw.node {
                        scene.push(SceneItem::Rect(SceneRect {
                            node,
                            rect: draw.rect,
                            color: draw.color,
                            border_radius: draw.border_radius,
                            opacity: draw.opacity,
                        }));
                    }
                }
                SceneCommand::DrawGlyphRun(draw) => {
                    if let Some(node) = draw.run.node {
                        scene.push(SceneItem::Text(SceneText {
                            node,
                            rect: draw.run.rect,
                            text: draw.run.text.clone(),
                            color: draw.run.color,
                            style: draw.run.style.clone(),
                            opacity: draw.run.opacity,
                        }));
                    }
                }
                SceneCommand::PushClip(_)
                | SceneCommand::PopClip
                | SceneCommand::PushTransform(_)
                | SceneCommand::PopTransform
                | SceneCommand::SaveLayer { .. }
                | SceneCommand::RestoreLayer => {}
            }
        }
        scene
    }

    /// Find the first glyph-run draw command with exact text content.
    #[must_use]
    pub fn find_text(&self, expected: &str) -> Option<&DrawGlyphRun> {
        self.commands.iter().find_map(|command| match command {
            SceneCommand::DrawGlyphRun(draw) if draw.run.text == expected => Some(draw),
            _ => None,
        })
    }
}

/// Renderer scene opcode.
#[derive(Clone, Debug, PartialEq)]
pub enum SceneCommand {
    /// Push a clip rectangle for following commands.
    PushClip(Rect),
    /// Pop the current clip rectangle.
    PopClip,
    /// Push an affine transform for following commands.
    PushTransform(Transform),
    /// Pop the current affine transform.
    PopTransform,
    /// Start an opacity layer.
    SaveLayer {
        /// Layer opacity in `[0, 1]`.
        opacity: f32,
    },
    /// Restore the current opacity layer.
    RestoreLayer,
    /// Draw a filled rectangle.
    DrawRect(DrawRect),
    /// Draw a glyph run.
    DrawGlyphRun(DrawGlyphRun),
}

/// Fill style for a rectangle draw command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SceneRectStyle {
    /// Resolved fill colour.
    pub color: Rgba8,
    /// Optional corner radius.
    pub border_radius: Option<Px>,
    /// Opacity encoded as `value * 10000`.
    pub opacity: u16,
}

impl SceneRectStyle {
    /// Create a rectangle fill style.
    #[must_use]
    pub const fn new(color: Rgba8, border_radius: Option<Px>, opacity: u16) -> Self {
        Self {
            color,
            border_radius,
            opacity,
        }
    }
}

/// Filled rectangle draw payload.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawRect {
    /// Source node, when emitted from a core tree.
    pub node: Option<NodeId>,
    /// Rectangle in scene logical pixels.
    pub rect: Rect,
    /// Resolved fill colour.
    pub color: Rgba8,
    /// Optional corner radius.
    pub border_radius: Option<Px>,
    /// Opacity encoded as `value * 10000`.
    pub opacity: u16,
}

impl DrawRect {
    /// Create a rectangle draw payload.
    #[must_use]
    pub const fn new(rect: Rect, style: SceneRectStyle) -> Self {
        Self {
            node: None,
            rect,
            color: style.color,
            border_radius: style.border_radius,
            opacity: style.opacity,
        }
    }

    pub(crate) const fn with_node(mut self, node: NodeId) -> Self {
        self.node = Some(node);
        self
    }
}

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
