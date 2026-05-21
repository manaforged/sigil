//! Sigil application shell — one-call `launch` that hides the winit /
//! wgpu / layout / router plumbing every app otherwise rewrites by hand.
//!
//! ```text
//! fn main() -> app::AppResult<()> {
//!     app::launch("hello-style", |cx| app_tree(cx))
//! }
//! ```
//!
//! See DR-016 (app launch) and the `phase-08a-ergonomics-pass.md` plan.

// wgpu-hal 29.0.1 GLES / WGL backend expects
// `impl std::error::Error for windows_core::Error` gated behind
// windows-core's `std` feature. Features are additive; enabling the
// dep here unifies it across the dependency graph for binaries that
// pull sigil-app in.
#[cfg(windows)]
use windows_core as _;

// Transitive deps threaded through launch; names are used there.
use style as _;

mod config;
mod error;
mod launch;
mod text_measure;
mod tree_root;

pub use config::{AppConfig, PanicPolicy};
pub use error::{AppResult, Error};
pub use launch::launch;
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
