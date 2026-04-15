//! Hardware abstraction layer for the sigil workspace.
//!
//! Every downstream crate depends on the traits defined here, never on
//! upstream crates like `wgpu`, `winit`, `taffy`, `parley`, or
//! `accesskit` directly. This insulates the library from breaking
//! releases, provides test seams (CPU/headless implementations), and
//! gives a single chokepoint where each upstream's API quirks are
//! normalised.
//!
//! See the workspace decision records in `docs/decisions/` for the
//! rationale behind each trait.
//!
//! # Status: Proposed (Phase 0 sketches, not yet validated)
//!
//! **These trait shapes are sketches, not load-bearing contracts.**
//!
//! Phase 0 ships the trait names, the decision records behind them,
//! and a minimum of shape to make the crate compile. No real adapter
//! implements any of these traits yet — `hello-rect` deliberately
//! uses `winit` and `wgpu` directly so that Phase 1 can let an actual
//! `Renderer`/`Shell` implementation inform the real shape rather than
//! designing speculatively.
//!
//! Concretely, every trait in this crate will change in Phase 1.1 when
//! `ui-renderer` and `ui-shell` land with working adapters. Do not build
//! downstream crates that depend on any specific method signature here
//! until Phase 1.1 is accepted. The invariants documented in the DRs
//! (`docs/decisions/`) are load-bearing; the method signatures in this
//! crate are provisional.

#![no_std]

pub mod a11y;
pub mod geometry;
pub mod layout;
pub mod renderer;
pub mod scheduler;
pub mod shell;
pub mod text;

pub use a11y::A11yBridge;
pub use geometry::{Color, PhysicalSize, Point, Rect};
pub use layout::Layout;
pub use renderer::Renderer;
pub use scheduler::Scheduler;
pub use shell::Shell;
pub use text::TextEngine;
