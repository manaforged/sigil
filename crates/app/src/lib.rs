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
use sigil_reactive as _;
use sigil_style as _;

// Facade dev-dep used by doctests and integration tests, not by unit tests.
#[cfg(test)]
use sigil as _;

mod config;
/// Native file dialogs (open / save / folder).
#[cfg(feature = "desktop")]
pub mod dialog;
mod embed;
mod error;
#[cfg(feature = "desktop")]
mod launch;
/// Native menu bar.
#[cfg(feature = "desktop")]
pub mod menu;
mod paint;
#[cfg(feature = "desktop")]
mod text_geometry;
#[cfg(feature = "desktop")]
mod text_measure;
#[cfg(feature = "desktop")]
mod theme_file;
/// System tray icon.
#[cfg(feature = "desktop")]
pub mod tray;
mod tree_root;

pub use config::{AppConfig, PanicPolicy};
pub use embed::{Host, Painted};
pub use error::{AppResult, Error};
#[cfg(feature = "desktop")]
pub use launch::launch;
pub use sigil_hal::{Anchor, WindowOpts};
pub use sigil_layout::LayoutError;
pub use sigil_profile as profile;
#[cfg(feature = "desktop")]
pub use sigil_renderer::{TextTuning, set_text_tuning, text_tuning};
#[cfg(feature = "desktop")]
pub use sigil_text::{DefaultFonts, set_default_fonts, use_system_fonts};
#[cfg(feature = "desktop")]
pub use text_geometry::EngineGeometry;
#[cfg(feature = "desktop")]
pub use text_measure::DefaultTextMeasure;
pub use tree_root::TreeRoot;

/// G5 (DR-048 §5) — install the production `TaskExecutor` +
/// `MainThreadDispatcher` pair against an `AsyncRuntime`.
///
/// Called by `launch::run` at startup; re-exported for the
/// `g5_executor` integration test which drives the same install path
/// without booting winit/wgpu.
#[cfg(feature = "desktop")]
pub use launch::install_g5_executor;

/// Test-only re-export of the frame driver's phase-transition shape.
/// See [`launch::__drive_test_frame`] for the contract.
#[cfg(feature = "desktop")]
#[doc(hidden)]
pub use launch::__drive_test_frame;

#[cfg(not(feature = "desktop"))]
mod desktop_unused {
    use accesskit as _;
    use crossbeam_channel as _;
    use notify as _;
    use sigil_a11y as _;
    use sigil_animation_runtime as _;
    use sigil_tasks as _;
    use toml as _;
    use tracing as _;
}
