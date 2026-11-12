#[cfg(host)]
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

#[cfg(host)]
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

#[cfg(host)]
mod blur_paint;
#[cfg(host)]
mod blur_shader;
#[cfg(host)]
mod cache_limits;
#[cfg(host)]
#[cfg(feature = "scene")]
#[cfg(any(terminal_host, feature = "inspect"))]
pub mod capture;
pub(crate) mod color_glyph_paint;
pub(crate) const SRGB_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
#[cfg(app_host)]
/// The opaque color a window clears to before it paints.
pub(crate) const SURFACE_CLEAR: crate::style::Rgba = crate::style::Rgba::new(0.03, 0.04, 0.05, 1.0);

#[cfg(all(feature = "cpu", host))]
mod cpu;
#[cfg(host)]
pub(crate) mod damage;
#[cfg(host)]
mod decoration_paint;
#[cfg(host)]
mod device_state;
#[cfg(host)]
#[cfg(target_arch = "wasm32")]
pub(crate) use device_state::{DeviceParts, OnLost};
#[cfg(host)]
mod fill_paint;
#[cfg(host)]
mod filter;
#[cfg(host)]
pub(crate) mod frame_stats;
#[cfg(host)]
mod frame_upload;
#[cfg(host)]
mod glyph_place;
#[cfg(host)]
mod layer_composite;
#[cfg(host)]
mod layer_pool;
#[cfg(host)]
pub(crate) mod offscreen;
#[cfg(host)]
mod plan;
#[cfg(host)]
mod projected;
#[cfg(host)]
mod quad_paint;
#[cfg(host)]
mod raster;
#[cfg(host)]
#[allow(clippy::module_inception)]
mod renderer;
#[cfg(host)]
mod scene_command;
#[cfg(host)]
mod scene_instance;
#[cfg(host)]
mod scene_paint;
#[cfg(host)]
#[cfg(feature = "kittest")]
pub mod scenes;
#[cfg(host)]
mod scissor;
#[cfg(host)]
mod shadow_paint;
pub(crate) mod slug;
#[cfg(host)]
mod split_encoder;
#[cfg(host)]
mod sprite_atlas;
#[cfg(host)]
mod surface;
#[cfg(host)]
#[cfg(feature = "kittest")]
mod test_state;
#[cfg(host)]
#[cfg(test)]
#[path = "renderer/lib_tests.rs"]
mod tests;
pub(crate) mod vector;
#[cfg(host)]
mod vector_pass;
mod wash_cache;
#[cfg(host)]
mod window_handle;

#[cfg(host)]
#[cfg(feature = "cpu")]
pub use cpu::CpuRenderer;
#[cfg(host)]
#[cfg(feature = "cpu")]
pub use cpu::CpuSurface;
#[cfg(host)]
#[cfg(feature = "kittest")]
pub use frame_stats::FrameStats;
#[cfg(host)]
#[cfg(feature = "kittest")]
pub use layer_pool::{LayerHandle, LayerPool};
#[cfg(host)]
pub use offscreen::OffscreenTarget;
#[cfg(host)]
#[cfg(feature = "kittest")]
pub use quad_paint::BUILTIN_SPRITE_ATLAS_ID;
#[cfg(host)]
#[cfg(feature = "kittest")]
pub use quad_paint::atlas_size;
#[cfg(host)]
#[cfg(feature = "kittest")]
pub use renderer::ScreenSubmission;
#[cfg(host)]
pub use renderer::WgpuRenderer;
#[cfg(host)]
#[cfg(feature = "kittest")]
pub use slug::{DrawParams, SlugTextPipeline};
#[cfg(host)]
pub use surface::WgpuSurface;
#[cfg(feature = "kittest")]
pub use test_state::{GlobalTextState, lock_global_text_state};
#[cfg(host)]
pub use window_handle::WindowHandle;
#[cfg(feature = "desktop")]
pub(crate) use window_handle::into_window_handle;
