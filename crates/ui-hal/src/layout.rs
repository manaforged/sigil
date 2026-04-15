//! Layout trait — see [DR-003].
//!
//! The `Layout` trait abstracts the box-layout engine. In production
//! this is implemented over `taffy` upstream. Tests implement a fixed
//! layout that returns predetermined rectangles.
//!
//! # Phase 0 shape
//!
//! The trait is a placeholder. Phase 1 defines `NodeId`, `Style`, and
//! `LayoutResult` in `ui-core`, at which point the trait shape becomes
//! concrete. The placeholder exists so downstream crates can begin
//! depending on `ui_hal::Layout` without committing to a specific
//! engine's types.
//!
//! [DR-003]: ../../../docs/decisions/DR-003-layout-taffy.md

use core::error::Error;

/// Box-layout engine abstraction.
pub trait Layout {
    /// Error type returned by layout operations.
    type Error: Error + Send + Sync + 'static;

    /// Opaque identifier for a layout node.
    ///
    /// The engine may use its own id internally; `ui-core` maps its
    /// `NodeId`s to these through the adapter.
    type NodeId: Copy + Eq + core::hash::Hash;
}
