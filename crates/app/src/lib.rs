//! Sigil application shell — one-call `launch` that hides the winit /
//! wgpu / layout / router plumbing every app otherwise rewrites by hand.
//!
//! ```text
//! fn main() -> sigil_app::AppResult<()> {
//!     sigil_app::launch("hello-style", |cx| app_tree(cx))
//! }
//! ```
//!
//! See DR-016 (app launch) and the `phase-08a-ergonomics-pass.md` plan.

#![deny(missing_docs)]
// wgpu-hal 29.0.1 GLES / WGL backend expects
// `impl std::error::Error for windows_core::Error` gated behind
// windows-core's `std` feature. Features are additive; enabling the
// dep here unifies it across the dependency graph for binaries that
// pull sigil-app in.
#[cfg(windows)]
use windows_core as _;

// Transitive deps threaded through launch; names are used there.
use sigil_style as _;

mod config;
/// Native file dialogs (open / save / folder).
pub mod dialog;
mod embed;
mod error;
mod launch;
/// Native menu bar.
pub mod menu;
mod text_geometry;
mod text_measure;
mod theme_file;
/// System tray icon.
pub mod tray;
mod tree_root;

pub use config::{AppConfig, PanicPolicy};
pub use embed::UiContext;
pub use error::{AppResult, Error};
pub use launch::launch;
pub use sigil_hal::{Anchor, WindowOpts};
pub use sigil_renderer::{TextTuning, set_text_tuning, text_tuning};
pub use sigil_text::{DefaultFonts, set_default_fonts, use_system_fonts};
pub use text_geometry::EngineGeometry;
pub use text_measure::DefaultTextMeasure;
pub use tree_root::TreeRoot;

/// G5 (DR-048 §5) — install the production `TaskExecutor` +
/// `MainThreadDispatcher` pair against an `AsyncRuntime`.
///
/// Called by `launch::run` at startup; re-exported for the
/// `g5_executor` integration test which drives the same install path
/// without booting winit/wgpu.
pub use launch::install_g5_executor;

/// Test-only re-export of the frame driver's phase-transition shape.
/// See [`launch::__drive_test_frame`] for the contract.
#[doc(hidden)]
pub use launch::__drive_test_frame;
