//! Command-buffer scene representation.

#![allow(clippy::redundant_pub_crate)]

use crate::corner_radii::CornerRadii;
use crate::scene::{Scene, SceneItem, SceneRect, SceneText};
use crate::shadow_paint::Shadow;

pub(crate) mod adapter;
mod draw_handlers;
pub(crate) mod effects;
pub(crate) mod fills;
mod glyph;
mod layer_walk;
pub(crate) mod payloads;
pub(crate) mod walk;

pub use effects::{DecorationKind, LayerEffect, LayerScope, SpriteKind, SpriteSource, Transform};
pub use fills::{
    Fill, GradientStop, LinearGradient, PatternKind, Stroke, StrokeAlign, StrokeStyle,
};
pub use glyph::{DrawGlyphRun, GlyphRun};
pub use payloads::{
    DrawDecoration, DrawGradient, DrawPattern, DrawRect, DrawSprite, SceneDecorationStyle,
    SceneGradientStyle, ScenePatternStyle, SceneRectStyle, SceneSpriteStyle,
};

/// A scene represented as ordered drawing and state commands.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommandScene {
    /// The ordered scene commands.
    pub commands: Vec<SceneCommand>,
}

impl CommandScene {
    /// Create a command scene from raw commands.
    #[must_use]
    pub(crate) const fn new(commands: Vec<SceneCommand>) -> Self {
        Self { commands }
    }

    /// Convert this command scene back to the legacy flat scene shape.
    #[deprecated(note = "migrate to CommandScene walk; removed in Phase 12")]
    #[must_use]
    pub fn to_flat(&self) -> Scene {
        let mut scene = Scene::default();
        for command in &self.commands {
            match command {
                SceneCommand::Rect(draw) => {
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
                SceneCommand::GlyphRun(draw) => {
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
                SceneCommand::Gradient(_)
                | SceneCommand::Pattern(_)
                | SceneCommand::DrawShadow { .. }
                | SceneCommand::Sprite(_)
                | SceneCommand::Decoration(_)
                | SceneCommand::PushClip(_)
                | SceneCommand::PopClip
                | SceneCommand::PushTransform(_)
                | SceneCommand::PopTransform
                | SceneCommand::SaveLayer { .. }
                | SceneCommand::RestoreLayer
                | SceneCommand::PushLayer(_)
                | SceneCommand::PopLayer => {}
            }
        }
        scene
    }

    /// Find the first glyph-run draw command with exact text content.
    #[must_use]
    pub fn find_text(&self, expected: &str) -> Option<&DrawGlyphRun> {
        self.commands.iter().find_map(|command| match command {
            SceneCommand::GlyphRun(draw) if draw.run.text == expected => Some(draw),
            _ => None,
        })
    }
}

/// Renderer scene opcode.
#[derive(Clone, Debug, PartialEq)]
pub enum SceneCommand {
    /// Push a clip rectangle for following commands.
    PushClip(sigil_hal::Rect),
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
    /// Begin an offscreen layer scope.
    ///
    /// Commands until the matching [`PopLayer`](SceneCommand::PopLayer) are
    /// rendered into an offscreen texture and then composited back.
    PushLayer(LayerScope),
    /// End the most-recently-opened offscreen layer scope.
    PopLayer,
    /// Draw a filled rectangle.
    Rect(DrawRect),
    /// Draw a linear-gradient-filled rectangle.
    Gradient(DrawGradient),
    /// Draw a pattern-filled rectangle.
    Pattern(DrawPattern),
    /// Draw a drop shadow for a rounded rectangle.
    DrawShadow {
        /// Original casting surface `[x, y, w, h]` in scene logical pixels.
        rect: [f32; 4],
        /// Corner radii for the casting surface.
        radii: CornerRadii,
        /// Shadow parameters.
        shadow: Shadow,
    },
    /// Draw a glyph run.
    GlyphRun(DrawGlyphRun),
    /// Draw a sprite from an atlas (mono / subpixel / poly).
    Sprite(DrawSprite),
    /// Draw a text decoration (underline / strikethrough / wavy underline).
    Decoration(DrawDecoration),
}
