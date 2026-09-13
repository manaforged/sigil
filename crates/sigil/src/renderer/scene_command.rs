#![allow(clippy::redundant_pub_crate)]

use crate::renderer::corner_radii::CornerRadii;
use crate::renderer::shadow_paint::Shadow;

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
pub(crate) use glyph::shape_glyph_run_with;
pub use glyph::{DrawGlyphRun, GlyphRun, shape_glyph_run};
pub use payloads::{
    DrawDecoration, DrawGradient, DrawImage, DrawPattern, DrawRect, DrawSprite,
    SceneDecorationStyle, SceneGradientStyle, ScenePatternStyle, SceneRectStyle, SceneSpriteStyle,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommandScene {
    pub commands: Vec<SceneCommand>,
}

impl CommandScene {
    #[must_use]
    pub(crate) const fn new(commands: Vec<SceneCommand>) -> Self {
        Self { commands }
    }

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
