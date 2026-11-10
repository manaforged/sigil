#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
macro_rules! viewport_uniforms {
    () => {
        "
struct Uniforms {
    viewport: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
"
    };
}

#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
macro_rules! vertex_in {
    () => {
        "
struct VertexIn {
    @location(0) unit: vec2<f32>,
};
"
    };
}

#[cfg(windows)]
use windows_core as _;

#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod blur_paint;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod blur_shader;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod cache_limits;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
#[cfg(feature = "scene")]
#[cfg(any(test, feature = "inspect", feature = "kittest", feature = "terminal"))]
pub mod capture;
pub(crate) mod color_glyph_paint;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
/// The opaque color a window clears to before it paints.
pub(crate) const SURFACE_CLEAR: crate::style::Rgba = crate::style::Rgba::new(0.03, 0.04, 0.05, 1.0);

#[cfg(all(
    feature = "cpu",
    any(test, feature = "kittest", feature = "desktop", feature = "terminal")
))]
mod cpu;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
pub(crate) mod damage;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod decoration_paint;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod device_state;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
#[cfg(target_arch = "wasm32")]
pub(crate) use device_state::{DeviceParts, OnLost};
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod fill_paint;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod filter;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
pub(crate) mod frame_stats;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod frame_upload;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod glyph_place;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod layer_composite;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod layer_pool;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod oblique;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
pub(crate) mod offscreen;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod plan;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod projected;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod quad_paint;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod raster;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
#[allow(clippy::module_inception)]
mod renderer;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod scene_command;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod scene_instance;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod scene_paint;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
#[cfg(any(test, feature = "kittest"))]
pub mod scenes;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod scissor;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod shadow_paint;
pub(crate) mod slug;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod split_encoder;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod sprite_atlas;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod surface;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
#[cfg(any(test, feature = "kittest"))]
mod test_state;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
#[cfg(test)]
#[path = "renderer/lib_tests.rs"]
mod tests;
pub(crate) mod vector;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod vector_pass;
mod wash_cache;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
mod window_handle;

#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
#[cfg(feature = "cpu")]
pub use cpu::CpuRenderer;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
#[cfg(feature = "cpu")]
pub use cpu::CpuSurface;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
#[cfg(any(test, feature = "kittest"))]
pub use frame_stats::FrameStats;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
#[cfg(any(test, feature = "kittest"))]
pub use layer_pool::{LayerHandle, LayerPool};
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
pub use offscreen::OffscreenTarget;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
#[cfg(any(test, feature = "kittest"))]
pub use quad_paint::BUILTIN_SPRITE_ATLAS_ID;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
#[cfg(any(test, feature = "kittest"))]
pub use quad_paint::set_test_atlas_size_override;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
#[cfg(any(test, feature = "kittest"))]
pub use renderer::ScreenSubmission;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
pub use renderer::WgpuRenderer;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
#[cfg(any(test, feature = "kittest"))]
pub use slug::{DrawParams, SlugTextPipeline};
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
pub use surface::WgpuSurface;
#[cfg(any(test, feature = "kittest"))]
pub use test_state::{GlobalTextState, lock_global_text_state};
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
pub use window_handle::WindowHandle;
#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
pub(crate) use window_handle::into_window_handle;
