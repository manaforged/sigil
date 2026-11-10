#![allow(clippy::redundant_pub_crate)]

use crate::scene::corner_radii::CornerRadii;
use crate::scene::shadow::Shadow;

mod clip;
mod decoration;
pub(crate) mod effects;
pub(crate) mod fills;
mod glyph;
pub(crate) mod gradient;
pub(crate) mod image_fit;
pub(crate) mod payloads;
#[cfg(all(
    feature = "scene",
    any(test, feature = "kittest", feature = "desktop", feature = "terminal")
))]
pub(crate) mod stroke_band;
pub(crate) mod transform3d;
#[cfg(all(
    feature = "scene",
    any(test, feature = "kittest", feature = "desktop", feature = "terminal")
))]
pub(crate) mod trim;
mod wash;

pub use clip::Clip;
#[cfg(any(
    test,
    all(feature = "scene", feature = "terminal"),
    feature = "desktop",
    all(feature = "scene", feature = "kittest")
))]
pub(crate) use decoration::DecorationGeometry;
pub use effects::{
    BlendMode, DecorationKind, Filter, LayerEffect, LayerScope, Mask, MaskMode, SpriteKind,
    SpriteSource, Transform,
};
#[cfg(any(test, feature = "kittest"))]
pub use fills::PaintFill;
pub(crate) use fills::{FillRule, PathPaint};
pub use fills::{
    GradientStop, LinearGradient, PatternKind, Stroke, StrokeAlign, StrokeCap, StrokeJoin,
    StrokeStyle,
};
pub use glyph::GlyphRun;
pub(crate) use glyph::TextFlow;
#[cfg(all(
    feature = "scene",
    any(test, feature = "kittest", feature = "desktop", feature = "terminal")
))]
pub use glyph::shape_glyph_run;
pub(crate) use glyph::{RunInk, shape_run};
pub use gradient::{Gradient, GradientKind};
#[cfg(any(feature = "scene", feature = "terminal"))]
pub(crate) use payloads::MITER_LIMIT;
pub(crate) use payloads::{CellBorder, CellRect, DrawPath};
pub use payloads::{DrawDecoration, DrawGradient, DrawImage, DrawPattern, DrawRect, DrawSprite};
pub(crate) use payloads::{PathCmd, PathStroke, outline_key, rounded_outline};
#[cfg(all(
    feature = "scene",
    any(test, feature = "kittest", feature = "desktop", feature = "terminal")
))]
pub(crate) use stroke_band::{StrokeBand, StrokeDash};
pub use transform3d::Transform3D;
pub use wash::Wash;
#[cfg(all(
    feature = "scene",
    any(test, feature = "kittest", feature = "desktop", feature = "terminal")
))]
pub(crate) use wash::{MAX_PIGMENTS, centre_bits as wash_centre_bits};

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

    /// Returns the text runs of this scene in paint order.
    pub fn glyph_runs(&self) -> impl Iterator<Item = &GlyphRun> {
        self.commands.iter().filter_map(|command| match command {
            SceneCommand::GlyphRun(draw) => Some(draw),
            _ => None,
        })
    }

    /// Returns the first text run whose text equals `expected`.
    #[must_use]
    pub fn find_text(&self, expected: &str) -> Option<&GlyphRun> {
        self.glyph_runs().find(|draw| draw.text == expected)
    }
}

/// One draw or state command in a `CommandScene`.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum SceneCommand {
    /// Clips the commands up to the matching `PopClip`.
    PushClip(Clip),
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
    /// The vector path scene command.
    Path(DrawPath),
    /// The decoration scene command.
    Decoration(DrawDecoration),
}
