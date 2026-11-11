#[cfg(window_host)]
pub(crate) mod backdrop;
pub(crate) mod builder;
pub(crate) mod canvas;
pub(crate) mod command;
pub(crate) mod corner_radii;
#[cfg(scene_emit)]
pub(crate) mod damage;
pub(crate) mod emit;
pub(crate) mod emit_cache;
#[cfg(scene_emit)]
pub(crate) mod geometry;
#[cfg(vector_paint)]
pub(crate) mod path_raster;
pub(crate) mod shadow;
#[cfg(scene_emit)]
pub(crate) mod state;
pub(crate) mod style_adapter;

#[cfg(window_host)]
kittest_pub! {
    #[doc(hidden)]
    pub use backdrop::paint_backdrop;
}
pub use builder::SceneBuilder;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use command::PaintFill;
#[cfg(scene_host)]
kittest_pub! {
    #[doc(hidden)]
    pub use command::shape_glyph_run;
}
pub use command::{
    BlendMode, CommandScene, DrawRect, GlyphRun, SharedGlyph, StrokeCap, StrokeJoin, Wash,
};
kittest_pub! {
    #[doc(hidden)]
    pub use command::{
        Clip, DecorationKind, DrawDecoration, DrawGradient, DrawImage, DrawPath,
        DrawPattern, Filter, Gradient, GradientKind, GradientStop, LayerEffect,
        LayerScope, LinearGradient, Mask, MaskMode, PatternKind, SceneCommand,
        Stroke, StrokeAlign, StrokeStyle, Transform, Transform3D,
    };
}
kittest_pub! {
    #[doc(hidden)]
    #[cfg(scene_paint)]
    pub use command::{DrawSprite, SpriteKind, SpriteSource};
}
kittest_pub! {
    #[doc(hidden)]
    pub use command::{CellBorder, TextFlow};
}
kittest_pub! {
    #[doc(hidden)]
    pub use corner_radii::CornerRadii;
}
#[cfg(terminal_host)]
kittest_pub! {
    #[doc(hidden)]
    pub use emit::EmitDiff;
}
kittest_pub! {
    #[doc(hidden)]
    pub use emit::emit_scene;
}
pub use shadow::Shadow;
kittest_pub! {
    #[doc(hidden)]
    pub use shadow::ShadowKind;
}
