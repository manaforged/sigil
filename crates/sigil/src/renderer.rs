#[cfg(windows)]
use windows_core as _;

mod blur_paint;
mod blur_shader;
#[cfg(feature = "capture")]
pub mod capture;
pub(crate) mod color_glyph_paint;
#[cfg(feature = "cpu")]
mod cpu;
mod decoration_paint;
mod device_state;
mod error;
mod fill_paint;
mod frame_stats;
mod frame_upload;
mod layer_composite;
mod layer_pool;
mod oblique;
mod offscreen;
mod raster;
mod raster_paint;
#[allow(clippy::module_inception)]
mod renderer;
mod scene_command;
mod scene_instance;
mod scene_paint;
pub mod scenes;
mod scissor;
mod shadow_paint;
mod slug;
mod split_encoder;
mod sprite_atlas;
mod sprite_paint;
pub mod style;
mod surface;
#[cfg(any(test, feature = "kittest"))]
mod test_state;
#[cfg(test)]
#[path = "renderer/lib_tests.rs"]
mod tests;
mod window_handle;

pub use crate::scene::shape_glyph_run;
pub use crate::scene::{
    CommandScene, CornerRadii, DecorationKind, DrawDecoration, DrawGlyphRun, DrawGradient,
    DrawImage, DrawPattern, DrawRect, DrawSprite, GlyphRun, GradientStop, LayerEffect, LayerScope,
    LinearGradient, PaintFill, PatternKind, SceneBuilder, SceneCommand, SceneDecorationStyle,
    SceneGradientStyle, ScenePatternStyle, SceneRectStyle, SceneSpriteStyle, Shadow, ShadowKind,
    SpriteKind, SpriteSource, Stroke, StrokeAlign, StrokeStyle, TextFlow, Transform, emit_scene,
    emit_scene_clipped, unsupported_style_warning_count_for,
};
#[cfg(feature = "cpu")]
pub use cpu::{CpuRenderer, CpuSurface};
pub use error::RendererError;
pub use frame_stats::FrameStats;
pub use layer_pool::{LayerError, LayerHandle, LayerPool};
pub use offscreen::OffscreenTarget;
#[cfg(any(test, feature = "kittest"))]
pub use raster_paint::set_test_atlas_size_override;
pub use renderer::{ScreenSubmission, WgpuRenderer, WorldSubmission, submit_clear};
pub use slug::{DrawParams, SlugTextPipeline, TextTuning, set_text_tuning, text_tuning};
pub use sprite_paint::BUILTIN_SPRITE_ATLAS_ID;
pub use surface::WgpuSurface;
#[cfg(any(test, feature = "kittest"))]
pub use test_state::{GlobalTextState, lock_global_text_state};
pub use window_handle::{WindowHandle, into_window_handle};

pub mod prelude {
    pub use crate::renderer::style::{Decoration, Gradient, Pattern, Rect, Sprite};
    pub use crate::renderer::{CommandScene, CornerRadii, SceneBuilder, SceneCommand, Transform};
    pub use crate::renderer::{
        GlyphRun, LayerEffect, LayerScope, PaintFill, SpriteKind, SpriteSource,
    };
    pub use crate::renderer::{Shadow, ShadowKind};
}
