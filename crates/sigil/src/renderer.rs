#[cfg(windows)]
use windows_core as _;

mod blur_paint;
mod blur_shader;
mod cache_limits;
#[cfg(feature = "scene")]
pub mod capture;
pub(crate) mod color_glyph_paint;
#[cfg(feature = "cpu")]
mod cpu;
mod decoration_paint;
mod device_state;
#[cfg(target_arch = "wasm32")]
pub(crate) use device_state::{DeviceParts, OnLost};
mod error;
mod fill_paint;
pub(crate) mod frame_stats;
mod frame_upload;
mod glyph_place;
mod layer_composite;
mod layer_pool;
mod oblique;
pub(crate) mod offscreen;
mod raster;
mod raster_paint;
#[allow(clippy::module_inception)]
mod renderer;
mod scene_command;
mod scene_instance;
mod scene_paint;
#[cfg(any(test, feature = "kittest"))]
pub mod scenes;
mod scissor;
mod shadow_paint;
pub(crate) mod slug;
mod split_encoder;
mod sprite_atlas;
mod sprite_paint;
mod surface;
#[cfg(any(test, feature = "kittest"))]
mod test_state;
#[cfg(test)]
#[path = "renderer/lib_tests.rs"]
mod tests;
mod window_handle;

#[cfg(feature = "cpu")]
pub use cpu::{CpuRenderer, CpuSurface};
pub use error::RendererError;
#[cfg(any(test, feature = "kittest"))]
pub use frame_stats::FrameStats;
pub use layer_pool::LayerError;
#[cfg(any(test, feature = "kittest"))]
pub use layer_pool::{LayerHandle, LayerPool};
pub use offscreen::OffscreenTarget;
#[cfg(any(test, feature = "kittest"))]
pub use raster_paint::set_test_atlas_size_override;
pub use renderer::{ScreenSubmission, WgpuRenderer};
#[cfg(any(test, feature = "kittest"))]
pub use slug::{DrawParams, SlugTextPipeline};
#[cfg(any(test, feature = "kittest"))]
pub use sprite_paint::BUILTIN_SPRITE_ATLAS_ID;
pub use surface::WgpuSurface;
#[cfg(any(test, feature = "kittest"))]
pub use test_state::{GlobalTextState, lock_global_text_state};
pub use window_handle::{WindowHandle, into_window_handle};
