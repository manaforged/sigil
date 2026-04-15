//! Text engine trait — see [DR-004].
//!
//! The `TextEngine` trait abstracts paragraph layout, shaping, caret
//! modelling, and selection geometry. In production this is
//! implemented over `parley` upstream. Tests implement a monospace
//! stub.
//!
//! Text editing operations (cursor movement, delete, undo/redo) and
//! IME integration are the responsibility of `ui-text`, not of this
//! trait. The trait exposes only what downstream crates need to lay
//! out and measure text for layout, hit-testing, and painting.
//!
//! # Phase 0 shape
//!
//! Placeholder. Phase 1 defines paragraph, run, glyph, and metrics
//! types. The trait exists here so downstream crates can name it
//! without committing to Parley types at their API surface.
//!
//! [DR-004]: ../../../docs/decisions/DR-004-text-parley.md

use core::error::Error;

/// Paragraph layout and shaping abstraction.
pub trait TextEngine {
    /// Error type returned by text engine operations.
    type Error: Error + Send + Sync + 'static;

    /// Opaque handle to a laid-out paragraph.
    type Paragraph;
}
