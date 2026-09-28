#![allow(clippy::redundant_pub_crate)]

use crate::scene::corner_radii::CornerRadii;
use crate::scene::shadow::Shadow;

mod decoration;
pub(crate) mod effects;
pub(crate) mod fills;
mod glyph;
#[cfg(feature = "scene")]
pub(crate) mod image_fit;
pub(crate) mod payloads;
#[cfg(feature = "scene")]
pub(crate) mod stroke_band;
#[cfg(feature = "scene")]
pub(crate) mod trim;

pub use decoration::DecorationGeometry;
pub use effects::{DecorationKind, LayerEffect, LayerScope, SpriteKind, SpriteSource, Transform};
pub use fills::{
    GradientStop, LinearGradient, PaintFill, PatternKind, Stroke, StrokeAlign, StrokeStyle,
};
#[cfg(feature = "scene")]
pub use glyph::shape_glyph_run;
pub use glyph::{DrawGlyphRun, GlyphRun, TextFlow};
pub(crate) use glyph::{RunInk, shape_run};
pub use payloads::{
    CellBorder, CellRect, DrawDecoration, DrawGradient, DrawImage, DrawPattern, DrawRect,
    DrawSprite, SceneDecorationStyle, SceneGradientStyle, ScenePatternStyle, SceneRectStyle,
    SceneSpriteStyle,
};
#[cfg(feature = "scene")]
pub(crate) use stroke_band::{StrokeBand, StrokeDash};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommandScene {
    pub commands: Vec<SceneCommand>,
}

impl CommandScene {
    #[must_use]
    pub(crate) const fn new(commands: Vec<SceneCommand>) -> Self {
        Self { commands }
    }

    #[cfg(any(test, feature = "kittest"))]
    #[must_use]
    pub fn find_text(&self, expected: &str) -> Option<&DrawGlyphRun> {
        self.commands.iter().find_map(|command| match command {
            SceneCommand::GlyphRun(draw) if draw.run.text == expected => Some(draw),
            _ => None,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SceneCommand {
    PushClip(crate::hal::Rect),
    PopClip,
    PushTransform(Transform),
    PopTransform,
    SaveLayer {
        opacity: f32,
    },
    RestoreLayer,
    PushLayer(LayerScope),
    PopLayer,
    Rect(DrawRect),
    Gradient(DrawGradient),
    Pattern(DrawPattern),
    DrawShadow {
        rect: [f32; 4],
        radii: CornerRadii,
        shadow: Shadow,
    },
    GlyphRun(DrawGlyphRun),
    Sprite(DrawSprite),
    Image(DrawImage),
    Decoration(DrawDecoration),
}
