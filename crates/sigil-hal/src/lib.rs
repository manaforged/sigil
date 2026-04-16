//! Hardware abstraction layer for Sigil.
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
//! # Status
//!
//! `Layout` became load-bearing when `sigil-layout` landed over Taffy.
//! The other trait shapes are still sketches until their first real
//! adapters harden them.

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
pub use layout::{Dim, Layout, Size};
pub use renderer::Renderer;
pub use scheduler::Scheduler;
pub use shell::Shell;
pub use text::{HitPosition, ParagraphMetrics, RenderedGlyph, TextAffinity, TextEngine};
