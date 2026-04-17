//! wgpu-backed [`Renderer`] adapter for Sigil.
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
use taffy as _;

mod error;
mod renderer;
mod scene;
mod slug;
mod style_adapter;
mod surface;
mod window_handle;

pub use error::RendererError;
pub use renderer::WgpuRenderer;
pub use scene::{Scene, SceneItem, SceneRect, SceneText, emit_scene};
pub use slug::{DrawParams, SlugTextPipeline};
pub use style_adapter::unsupported_style_warning_count_for;
pub use surface::WgpuSurface;
pub use window_handle::{WindowHandle, into_window_handle};
