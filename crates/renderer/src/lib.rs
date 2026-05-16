//! wgpu-backed [`sigil_hal::Renderer`] adapter for Sigil.
//!
//! This crate is the single chokepoint where `wgpu` types live. Every
//! downstream crate names the [`sigil_hal::Renderer`] trait and treats the
//! GPU as an opaque implementation detail. See
//! [DR-001](../../../docs/decisions/DR-001-renderer-wgpu.md) for the
//! rationale and [the trait docs][`sigil_hal::Renderer`] for the Phase 2
//! surface area (create surface, resize, clear-and-present).
//!
//! # Usage
//!
//! ```text
//! use std::sync::Arc;
//!
//! use sigil_hal::{Color, PhysicalSize, Renderer};
//! use sigil_renderer::{WgpuRenderer, into_window_handle};
//!
//! fn run(window: Arc<impl raw_window_handle::HasWindowHandle
//!     + raw_window_handle::HasDisplayHandle
//!     + Send + Sync + 'static>) -> Result<(), Box<dyn std::error::Error>> {
//!     let mut renderer = WgpuRenderer::new()?;
//!     let mut surface = renderer.create_surface(
//!         into_window_handle(window),
//!         PhysicalSize::new(720, 480),
//!     )?;
//!     renderer.clear(&mut surface, Color::BLACK)?;
//!     Ok(())
//! }
//! ```
//!
//! The block above is rendered as `text` rather than runnable Rust
//! because the example needs a live windowed handle to be meaningful;
//! `hello-rect` (once ported) is the executable reference instead.

// wgpu-hal 29.0.1 (GLES / WGL backend) expects
// `impl std::error::Error for windows_core::Error`, which is gated
// behind `windows-core`'s `std` feature. Features are additive across a
// dep graph, so enabling the crate here with `std` unifies it across
// the whole build. This crate does not use `windows-core` directly.
#[cfg(windows)]
use windows_core as _;

#[cfg(test)]
use criterion as _;
#[cfg(test)]
use half as _;
#[cfg(test)]
use image as _;
#[cfg(test)]
use proptest as _;
#[cfg(test)]
use sigil_shell as _;
#[cfg(test)]
use smallvec as _;
#[cfg(test)]
use taffy as _;

mod blur_paint;
mod blur_shader;
mod corner_radii;
mod decoration_paint;
mod device_state;
mod error;
mod fill_paint;
mod layer_composite;
mod layer_pool;
mod offscreen;
mod renderer;
mod scene;
mod scene_builder;
mod scene_command;
mod scene_instance;
mod scene_paint;
pub mod scenes;
mod shadow_paint;
mod slug;
mod sprite_atlas;
mod sprite_paint;
pub mod style;
mod style_adapter;
mod surface;
mod window_handle;

pub use corner_radii::CornerRadii;
pub use error::RendererError;
pub use layer_pool::{LayerHandle, LayerPool};
pub use offscreen::OffscreenTarget;
pub use renderer::{ScreenSubmission, WgpuRenderer, WorldSubmission, submit_clear};
#[doc(hidden)]
pub use scene::shape_glyph_run;
pub use scene::{Scene, SceneItem, SceneRect, SceneText, emit_scene};
pub use scene_builder::SceneBuilder;
pub use scene_command::{
    CommandScene, DecorationKind, DrawDecoration, DrawGlyphRun, DrawGradient, DrawPattern,
    DrawRect, DrawSprite, Fill, GlyphRun, GradientStop, LayerEffect, LayerScope, LinearGradient,
    PatternKind, SceneCommand, SceneDecorationStyle, SceneGradientStyle, ScenePatternStyle,
    SceneRectStyle, SceneSpriteStyle, SpriteKind, SpriteSource, Stroke, StrokeAlign, StrokeStyle,
    Transform,
};
pub use shadow_paint::{Shadow, ShadowKind};
pub use slug::{DrawParams, SlugTextPipeline};
#[doc(hidden)]
pub use sprite_paint::BUILTIN_SPRITE_ATLAS_ID;
pub use style_adapter::unsupported_style_warning_count_for;
pub use surface::WgpuSurface;
pub use window_handle::{WindowHandle, into_window_handle};

/// Convenience re-exports for widget and renderer authors.
///
/// `use sigil_renderer::prelude::*;` gives a typical scene-building session
/// all the types it needs without a 15-line import block.
pub mod prelude {
    pub use crate::style::{Decoration, Gradient, Pattern, Rect, Sprite};
    pub use crate::{CommandScene, CornerRadii, SceneBuilder, SceneCommand, Transform};
    pub use crate::{Fill, GlyphRun, LayerEffect, LayerScope, SpriteKind, SpriteSource};
    pub use crate::{Shadow, ShadowKind};
}
