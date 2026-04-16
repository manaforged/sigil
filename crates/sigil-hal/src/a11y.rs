//! Accessibility bridge trait — see [DR-008].
//!
//! The `A11yBridge` trait abstracts the OS accessibility surface. In
//! production this is implemented over `accesskit` upstream (with its
//! `tokio` feature to avoid the zbus blocking-API panic on Linux).
//! Tests implement a recording bridge that captures `TreeUpdate`s and
//! injects `ActionRequest`s for assertion.
//!
//! Accessibility is a Phase 1 concern in this project. Semantic
//! identity (role, name, description, actions, focus order, stable
//! `NodeId`) lives on the node model from the first commit — retrofit
//! is not an option. See DR-008 for the full reasoning.
//!
//! # Bidirectional from day one
//!
//! Accessibility is not outbound-only. Screen readers and assistive
//! tech dispatch actions *into* the app — invoke, focus, toggle,
//! increment, decrement, expand, collapse, scroll-to, set-value, and
//! text-edit requests — and those actions must land on the node that
//! authored them. Modeling the trait as publish-only is the retrofit
//! failure DR-008 says we must avoid: adding inbound dispatch later
//! touches `NodeId`, `Semantics`, focus management, and every
//! primitive's API surface.
//!
//! The trait therefore carries both surfaces from Phase 0:
//!
//! - `publish(update)` — outbound `TreeUpdate` to the platform adapter.
//! - `take_action()` — inbound `ActionRequest` drained on the main
//!   thread and routed to the owning node by stable id.
//!
//! # Phase 0 shape
//!
//! Sketch. Phase 1.1 refines the action-dispatch surface once
//! `sigil-core` ships the stable `NodeId` and the `Semantics` struct.
//! The intent captured here is non-negotiable; the exact method
//! signatures are provisional.
//!
//! [DR-008]: ../../../docs/decisions/DR-008-a11y-accesskit.md

use core::error::Error;
use core::hash::Hash;

/// Accessibility publication and action-dispatch abstraction.
///
/// Implementations are main-thread only. Platform adapters that run
/// on their own thread (e.g. Linux AT-SPI2 on a zbus task) forward
/// actions through a `Scheduler` dispatch so they surface on the main
/// thread before reaching `take_action`.
pub trait A11yBridge {
    /// Error type returned by bridge operations.
    type Error: Error + Send + Sync + 'static;

    /// Opaque `TreeUpdate` type — `accesskit::TreeUpdate` in production.
    type Update;

    /// Opaque `ActionRequest` type — `accesskit::ActionRequest` in
    /// production. Carries the action kind and any associated data
    /// (e.g. a string for `SetValue`, a point for `ScrollIntoView`).
    type Action;

    /// Stable per-node identity shared with `sigil-core`'s `NodeId`.
    ///
    /// Phase 0 keeps this an associated type so the crate does not
    /// have to pick an int width before `sigil-core` ships. Phase 1.1
    /// unifies this with the reactive owner tree's `NodeId`.
    type NodeId: Copy + Eq + Hash;

    /// Publish an outbound `TreeUpdate` to the platform adapter.
    ///
    /// The bridge atomically applies the update to its live tree. The
    /// caller is responsible for batching — one `publish` per frame
    /// is the Phase 1.1 target.
    /// # Errors
    ///
    /// Returns an error when the platform bridge cannot accept or
    /// apply the update.
    fn publish(&mut self, update: Self::Update) -> Result<(), Self::Error>;

    /// Drain the next queued inbound action request.
    ///
    /// Returns `None` when the queue is empty.
    ///
    /// # Future contract (Phase 1.1 a11y adapter)
    ///
    /// The Phase 0 trait carries this signature so consumers can name
    /// the inbound-action surface, but the trait itself does not
    /// validate node liveness, ordering, or non-blocking behaviour —
    /// none of those properties are expressible in the signature alone.
    /// The Phase 1.1 a11y adapter will enforce the following semantics;
    /// until that adapter ships, treat them as the intended contract
    /// rather than a trait-level guarantee:
    ///
    /// - Preserve enqueue order (actions arrive FIFO).
    /// - Drop actions whose target node has been disposed since the
    ///   action was enqueued. Identity is checked at drain time
    ///   against the current owner tree.
    /// - Never block. `take_action` is polled at the start of each
    ///   frame by the shell.
    fn take_action(&mut self) -> Option<(Self::NodeId, Self::Action)>;
}
