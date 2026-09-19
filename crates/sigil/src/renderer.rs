#[cfg(windows)]
use windows_core as _;

mod blur_paint;
mod blur_shader;
#[cfg(feature = "capture")]
pub mod capture;
mod color_glyph_paint;
mod corner_radii;
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
mod offscreen;
mod raster;
mod raster_paint;
#[allow(clippy::module_inception)]
mod renderer;
mod scene;
mod scene_builder;
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
mod style_adapter;
mod surface;
mod test_state;
#[cfg(test)]
#[path = "renderer/lib_tests.rs"]
mod tests;
mod window_handle;

pub use corner_radii::CornerRadii;
#[cfg(feature = "cpu")]
pub use cpu::{CpuRenderer, CpuSurface};
pub use error::RendererError;
pub use frame_stats::FrameStats;
pub use layer_pool::{LayerError, LayerHandle, LayerPool};
pub use offscreen::OffscreenTarget;
pub use raster_paint::set_test_atlas_size_override;
pub use renderer::{ScreenSubmission, WgpuRenderer, WorldSubmission, submit_clear};
#[doc(hidden)]
pub use scene_command::shape_glyph_run;
mod emit_cache;
pub use scene::{emit_scene, emit_scene_clipped};
pub use scene_builder::SceneBuilder;
pub use scene_command::{
    CommandScene, DecorationKind, DrawDecoration, DrawGlyphRun, DrawGradient, DrawImage,
    DrawPattern, DrawRect, DrawSprite, Fill, GlyphRun, GradientStop, LayerEffect, LayerScope,
    LinearGradient, PatternKind, SceneCommand, SceneDecorationStyle, SceneGradientStyle,
    ScenePatternStyle, SceneRectStyle, SceneSpriteStyle, SpriteKind, SpriteSource, Stroke,
    StrokeAlign, StrokeStyle, Transform,
};
pub use shadow_paint::{Shadow, ShadowKind};
pub use slug::{DrawParams, SlugTextPipeline, TextTuning, set_text_tuning, text_tuning};
#[doc(hidden)]
pub use sprite_paint::BUILTIN_SPRITE_ATLAS_ID;
pub use style_adapter::unsupported_style_warning_count_for;
pub use surface::WgpuSurface;
pub use test_state::{GlobalTextState, lock_global_text_state};
pub use window_handle::{WindowHandle, into_window_handle};

pub mod prelude {
    pub use crate::renderer::style::{Decoration, Gradient, Pattern, Rect, Sprite};
    pub use crate::renderer::{CommandScene, CornerRadii, SceneBuilder, SceneCommand, Transform};
    pub use crate::renderer::{Fill, GlyphRun, LayerEffect, LayerScope, SpriteKind, SpriteSource};
    pub use crate::renderer::{Shadow, ShadowKind};
}
