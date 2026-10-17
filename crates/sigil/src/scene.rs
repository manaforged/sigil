pub(crate) mod backdrop;
pub(crate) mod builder;
pub(crate) mod command;
pub(crate) mod corner_radii;
pub(crate) mod damage;
pub(crate) mod emit;
pub(crate) mod emit_cache;
pub(crate) mod geometry;
pub(crate) mod shadow;
pub(crate) mod state;
pub(crate) mod style_adapter;

pub use backdrop::paint_backdrop;
pub use builder::SceneBuilder;
#[cfg(feature = "scene")]
pub use command::shape_glyph_run;
pub use command::{
    CellBorder, CellRect, CommandScene, DecorationGeometry, DecorationKind, DrawDecoration,
    DrawGlyphRun, DrawGradient, DrawImage, DrawPattern, DrawRect, DrawSprite, GlyphRun,
    GradientStop, LayerEffect, LayerScope, LinearGradient, PaintFill, PatternKind, SceneCommand,
    SceneDecorationStyle, SceneGradientStyle, ScenePatternStyle, SceneRectStyle, SceneSpriteStyle,
    SpriteKind, SpriteSource, Stroke, StrokeAlign, StrokeStyle, TextFlow, Transform,
};
pub use corner_radii::CornerRadii;
pub use emit::{emit_scene, emit_scene_clipped};
pub use shadow::{Shadow, ShadowKind};
