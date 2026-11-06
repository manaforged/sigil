#[cfg(any(test, feature = "kittest", feature = "desktop"))]
pub(crate) mod backdrop;
pub(crate) mod builder;
pub(crate) mod command;
pub(crate) mod corner_radii;
#[cfg(any(
    test,
    feature = "terminal",
    feature = "desktop",
    all(feature = "scene", feature = "kittest")
))]
pub(crate) mod damage;
pub(crate) mod emit;
pub(crate) mod emit_cache;
#[cfg(any(
    test,
    feature = "terminal",
    feature = "desktop",
    all(feature = "scene", feature = "kittest")
))]
pub(crate) mod geometry;
#[cfg(any(feature = "scene", feature = "terminal"))]
pub(crate) mod path_raster;
pub(crate) mod shadow;
#[cfg(any(
    test,
    feature = "terminal",
    feature = "desktop",
    all(feature = "scene", feature = "kittest")
))]
pub(crate) mod state;
pub(crate) mod style_adapter;

#[cfg(any(test, feature = "kittest", feature = "desktop"))]
kittest_pub! {
    #[doc(hidden)]
    pub use backdrop::paint_backdrop;
}
pub use builder::SceneBuilder;
#[cfg(any(test, feature = "kittest"))]
#[doc(hidden)]
pub use command::PaintFill;
#[cfg(all(
    feature = "scene",
    any(test, feature = "kittest", feature = "desktop", feature = "terminal")
))]
kittest_pub! {
    #[doc(hidden)]
    pub use command::shape_glyph_run;
}
#[cfg(any(test, feature = "terminal"))]
pub(crate) use command::{CellBorder, TextFlow};
pub use command::{Clip, CommandScene, DrawRect, GlyphRun, SceneCommand, StrokeCap, StrokeJoin, Wash};
kittest_pub! {
    #[doc(hidden)]
    pub use command::Transform;
}
kittest_pub! {
    #[doc(hidden)]
    pub use command::Transform3D;
}
kittest_pub! {
    #[doc(hidden)]
    pub use command::{DecorationKind, DrawDecoration, DrawGradient, DrawImage, DrawPath, DrawPattern, DrawSprite, Gradient, GradientKind, GradientStop, LayerEffect, LayerScope, LinearGradient, Mask, MaskMode, PatternKind, SpriteKind, SpriteSource, Stroke, StrokeAlign, StrokeStyle};
}
kittest_pub! {
    #[doc(hidden)]
    pub use corner_radii::CornerRadii;
}
pub use emit::emit_scene;
pub use shadow::Shadow;
kittest_pub! {
    #[doc(hidden)]
    pub use shadow::ShadowKind;
}
