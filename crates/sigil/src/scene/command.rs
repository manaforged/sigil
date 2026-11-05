#![allow(clippy::redundant_pub_crate)]

use crate::scene::corner_radii::CornerRadii;
use crate::scene::shadow::Shadow;

mod decoration;
pub(crate) mod effects;
pub(crate) mod fills;
mod glyph;
pub(crate) mod image_fit;
pub(crate) mod payloads;
#[cfg(all(
    feature = "scene",
    any(test, feature = "kittest", feature = "desktop", feature = "terminal")
))]
pub(crate) mod stroke_band;
#[cfg(all(
    feature = "scene",
    any(test, feature = "kittest", feature = "desktop", feature = "terminal")
))]
pub(crate) mod trim;
mod wash;

#[cfg(any(
    test,
    all(feature = "scene", feature = "terminal"),
    feature = "desktop",
    all(feature = "scene", feature = "kittest")
))]
pub(crate) use decoration::DecorationGeometry;
pub use effects::{DecorationKind, LayerEffect, LayerScope, SpriteKind, SpriteSource, Transform};
#[cfg(any(test, feature = "kittest"))]
pub use fills::PaintFill;
pub use fills::{GradientStop, LinearGradient, PatternKind, Stroke, StrokeAlign, StrokeStyle};
pub use glyph::GlyphRun;
pub(crate) use glyph::TextFlow;
#[cfg(all(
    feature = "scene",
    any(test, feature = "kittest", feature = "desktop", feature = "terminal")
))]
pub use glyph::shape_glyph_run;
pub(crate) use glyph::{RunInk, shape_run};
pub(crate) use payloads::{CellBorder, CellRect};
pub use payloads::{DrawDecoration, DrawGradient, DrawImage, DrawPattern, DrawRect, DrawSprite};
pub use wash::{MAX_PIGMENTS, Wash};
#[cfg(all(
    feature = "scene",
    any(test, feature = "kittest", feature = "desktop", feature = "terminal")
))]
pub(crate) use stroke_band::{StrokeBand, StrokeDash};

/// The ordered drawing commands of one frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommandScene {
    /// The commands.
    pub commands: Vec<SceneCommand>,
}

impl CommandScene {
    #[must_use]
    pub(crate) const fn new(commands: Vec<SceneCommand>) -> Self {
        Self { commands }
    }

    #[cfg(any(test, feature = "kittest"))]
    #[must_use]
    pub fn find_text(&self, expected: &str) -> Option<&GlyphRun> {
        self.commands.iter().find_map(|command| match command {
            SceneCommand::GlyphRun(draw) if draw.text == expected => Some(draw),
            _ => None,
        })
    }
}

/// One draw or state command in a `CommandScene`.
#[derive(Clone, Debug, PartialEq)]
pub enum SceneCommand {
    /// The push clip scene command.
    PushClip(crate::geometry::Rect),
    /// The pop clip scene command.
    PopClip,
    /// The push transform scene command.
    PushTransform(Transform),
    /// The pop transform scene command.
    PopTransform,
    /// The save layer scene command.
    SaveLayer {
        /// The opacity.
        opacity: f32,
    },
    /// The restore layer scene command.
    RestoreLayer,
    /// The push layer scene command.
    PushLayer(LayerScope),
    /// The pop layer scene command.
    PopLayer,
    /// The rect scene command.
    Rect(DrawRect),
    /// The gradient scene command.
    Gradient(DrawGradient),
    /// The pattern scene command.
    Pattern(DrawPattern),
    /// The draw shadow scene command.
    DrawShadow {
        /// The rect.
        rect: [f32; 4],
        /// The radii.
        radii: CornerRadii,
        /// The shadow.
        shadow: Shadow,
    },
    /// The glyph run scene command.
    GlyphRun(GlyphRun),
    /// The sprite scene command.
    Sprite(DrawSprite),
    /// The image scene command.
    Image(DrawImage),
    /// The decoration scene command.
    Decoration(DrawDecoration),
}
