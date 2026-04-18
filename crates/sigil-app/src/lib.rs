//! Sigil application shell — one-call `launch` that hides the winit /
//! wgpu / layout / router plumbing every app otherwise rewrites by hand.
//!
//! ```text
//! fn main() -> sigil_app::Result<()> {
//!     sigil_app::launch("hello-style", |cx| app_tree(cx))
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
use sigil_async as _;
use sigil_style as _;

mod config;
mod error;
mod launch;
mod text_measure;
mod tree_root;

pub use config::{AppConfig, PanicPolicy};
pub use error::{Error, Result};
pub use launch::launch;
pub use text_measure::DefaultTextMeasure;
pub use tree_root::TreeRoot;
