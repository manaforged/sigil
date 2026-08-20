//! Hardware abstraction layer for Sigil.
//!
//! Every downstream crate depends on the traits defined here, never on
//! upstream crates like `wgpu`, `winit`, or `accesskit`
//! directly. This insulates the library from breaking
//! releases, provides test seams (CPU/headless implementations), and
//! gives a single chokepoint where each upstream's API quirks are
//! normalised.
//!
//! See the internal workspace decision records for the rationale
//! behind each trait.
//!
//! # Status
//!
//! `Dim` and `Size` are the layout constraint types. Production
//! layout is `sigil_layout::LayoutEngine`. Renderer, text, and
//! shell traits have production adapters.

#![no_std]
#![deny(missing_docs)]
extern crate alloc;

pub mod a11y;
pub mod events;
pub mod geometry;
pub mod layout;
pub mod renderer;
pub mod scheduler;
pub mod shell;
pub mod size;
pub mod text;

pub use a11y::A11yBridge;
pub use events::{
    FileDropEvent, FileDropPhase, ImeEvent, Key, KeyEvent, KeyEventInit, KeyPhase, LogicalKey,
    ModifiersState, MouseButton, MouseButtons, PhysicalKey, PointerEvent, PointerEventInit,
    PointerKind, PointerPhase, ScrollDelta, ScrollEvent, ScrollEventInit, ScrollPhase, ShellEvent,
    Shortcut, WindowEvent,
};
pub use geometry::{Color, PhysicalSize, Point, Rect};
pub use layout::{Dim, Size};
pub use renderer::Renderer;
pub use scheduler::Scheduler;
pub use shell::{Anchor, Shell, WindowOpts};
pub use size::{LogicalSize, Scale};
pub use text::{HitPosition, ParagraphMetrics, RenderedGlyph, TextAffinity, TextEngine};
