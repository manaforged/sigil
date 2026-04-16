//! Layout trait — see [DR-003].
//!
//! The `Layout` trait abstracts the box-layout engine. In production
//! this is implemented over `taffy` upstream. Tests implement a fixed
//! layout that returns predetermined rectangles.
//!
//! # Phase 4 shape
//!
//! [DR-003]: ../../../docs/decisions/DR-003-layout-taffy.md

use core::error::Error;

/// Available space in one layout axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dim {
    /// A definite length in logical pixels.
    Fixed(f32),
    /// Ask the engine for the minimum intrinsic size.
    MinContent,
    /// Ask the engine for the maximum intrinsic size.
    MaxContent,
}

impl Dim {
    /// Construct a fixed logical-pixel dimension.
    #[must_use]
    pub const fn fixed(value: f32) -> Self {
        Self::Fixed(value)
    }
}

/// Available space passed to layout computation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    /// Width constraint.
    pub width: Dim,
    /// Height constraint.
    pub height: Dim,
}

impl Size {
    /// Construct a definite logical-pixel size.
    #[must_use]
    pub const fn fixed(width: f32, height: f32) -> Self {
        Self {
            width: Dim::Fixed(width),
            height: Dim::Fixed(height),
        }
    }

    /// Construct a max-content size constraint.
    #[must_use]
    pub const fn max_content() -> Self {
        Self {
            width: Dim::MaxContent,
            height: Dim::MaxContent,
        }
    }

    /// Construct a min-content size constraint.
    #[must_use]
    pub const fn min_content() -> Self {
        Self {
            width: Dim::MinContent,
            height: Dim::MinContent,
        }
    }
}

/// Box-layout engine abstraction.
pub trait Layout {
    /// Error type returned by layout operations.
    type Error: Error + Send + Sync + 'static;

    /// Opaque identifier for a layout node.
    ///
    /// The engine may use its own id internally; `sigil-core` maps its
    /// `NodeId`s to these through the adapter.
    type NodeId: Copy + Eq + core::hash::Hash;

    /// Engine-specific style input.
    type Intent;

    /// Engine-specific computed output.
    type Result: Copy;

    /// Insert or update one layout node.
    ///
    /// # Errors
    ///
    /// Returns the backend error when the node cannot be inserted or
    /// updated.
    fn upsert(
        &mut self,
        id: Self::NodeId,
        intent: &Self::Intent,
    ) -> core::result::Result<(), Self::Error>;

    /// Replace a node's ordered children.
    ///
    /// # Errors
    ///
    /// Returns the backend error when `parent` or any child is unknown,
    /// invalid, or cannot be reparented.
    fn set_children(
        &mut self,
        parent: Self::NodeId,
        children: &[Self::NodeId],
    ) -> core::result::Result<(), Self::Error>;

    /// Remove a layout node and any descendants currently attached
    /// below it in the layout mirror.
    ///
    /// This updates only the layout backend. The caller still owns
    /// the authoritative UI tree and is responsible for removing or
    /// reparenting any corresponding core nodes there.
    ///
    /// # Errors
    ///
    /// Returns the backend error when `id` is unknown or removal fails.
    fn remove(&mut self, id: Self::NodeId) -> core::result::Result<(), Self::Error>;

    /// Compute layout for a root under the supplied available space.
    ///
    /// # Errors
    ///
    /// Returns the backend error when `root` is unknown or layout
    /// computation fails.
    fn compute(
        &mut self,
        root: Self::NodeId,
        available: Size,
    ) -> core::result::Result<(), Self::Error>;

    /// Return the most recently computed result for a node.
    fn result(&self, id: Self::NodeId) -> Option<Self::Result>;
}
