#![allow(unused_imports)]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

#[cfg(feature = "render")]
use sigil::render::{
    Color, CommandScene, DrawParams, OffscreenTarget, PhysicalSize, RenderedGlyph,
    ScreenSubmission, SlugTextPipeline, WgpuRenderer, WgpuSurface, emit_scene, submit_clear,
};
#[cfg(feature = "render")]
use sigil::text::{Font, FontCollection, OwnedTextEngine, Paragraph, TextConfig, TextError};

#[cfg(feature = "render")]
#[test]
fn render_surface_compiles() {}
