//! `A11yBridge` impls (DR-050 §3).
//!
//! Two impls land here:
//!
//! - [`RecordingBridge`] — the test-side bridge that stores published
//!   `TreeUpdate`s in a `Vec` and serves enqueued action requests from
//!   a FIFO. Tests pull the last published update to assert.
//! - [`AccessKitBridge`] — the production bridge. It does not own the
//!   `accesskit_winit::Adapter` itself (that stays shell-side, per
//!   DR-050 §1 — `sigil-a11y` does not depend on `winit`); instead it
//!   forwards `publish` through a caller-supplied sink closure and
//!   drains inbound actions from a `crossbeam-channel` receiver fed by
//!   the shell's `Send`-bound adapter handlers
//!   (`crates/shell/src/shell/a11y.rs`) — the shape
//!   `crates/hal/src/a11y.rs`'s doc comment anticipated.

use core::fmt;
use std::collections::VecDeque;

use accesskit::{ActionRequest, TreeUpdate};
use sigil_hal::A11yBridge;
use sigil_kernel::NodeId;

/// Test-side bridge — captures every `publish` into a `Vec` and
/// serves `take_action` from a FIFO queue tests prime with
/// [`RecordingBridge::action`].
///
/// Hidden from rustdoc: this is a test utility surfaced in the public
/// API for cross-crate tests to import, but it shouldn't appear in
/// rustdoc search alongside production types.
#[doc(hidden)]
#[derive(Debug, Default)]
pub struct RecordingBridge {
    /// Every successfully published `TreeUpdate`, in publish order.
    pub(crate) published: Vec<TreeUpdate>,
    /// FIFO of `(target_node_id, request)` pairs the bridge will hand
    /// back from `take_action`.
    pub pending_actions: VecDeque<(NodeId, ActionRequest)>,
}

impl RecordingBridge {
    /// Construct an empty bridge.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueue an AT-initiated action for the next drain.
    pub fn action(&mut self, target: NodeId, request: ActionRequest) {
        self.pending_actions.push_back((target, request));
    }

    /// Borrow the most recent `TreeUpdate`, if any.
    #[must_use]
    pub fn last_published(&self) -> Option<&TreeUpdate> {
        self.published.last()
    }

    /// Number of `TreeUpdate`s captured so far.
    ///
    /// Cross-crate tests use this to assert publish counts without
    /// having to borrow the private `published` vec directly.
    #[must_use]
    pub const fn published_count(&self) -> usize {
        self.published.len()
    }
}

impl A11yBridge for RecordingBridge {
    type Error = BridgeError;
    type Update = TreeUpdate;
    type Action = ActionRequest;
    type NodeId = NodeId;

    fn publish(&mut self, update: Self::Update) -> Result<(), Self::Error> {
        self.published.push(update);
        Ok(())
    }

    fn take_action(&mut self) -> Option<(Self::NodeId, Self::Action)> {
        self.pending_actions.pop_front()
    }
}

/// Error type for AccessKit bridge operations.
///
/// `RecordingBridge::publish` is infallible, but the [`A11yBridge`]
/// trait requires `Error: Error + Send + Sync + 'static`, so this
/// crate ships a concrete error type. No variant fires from the
/// recording path; the shell-side adapter wiring will surface real
/// failures as this type grows.
#[derive(Debug)]
pub struct BridgeError;

impl fmt::Display for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("accesskit bridge rejected update")
    }
}

impl core::error::Error for BridgeError {}

/// Production `A11yBridge` — forwards outbound `TreeUpdate`s through a
/// caller-supplied sink and drains inbound `ActionRequest`s from a
/// `crossbeam-channel` receiver.
///
/// `F` is generic (not `Box<dyn FnMut(TreeUpdate)>`) so the frame loop
/// can construct one per frame over a fresh `&mut WinitShell` borrow
/// with zero heap allocation — see `crates/app/src/launch.rs`'s
/// `publish_a11y` helper. The bridge itself is cheap to construct: the
/// `Receiver` clone is an `Arc` bump, not a channel allocation.
pub struct AccessKitBridge<F: FnMut(TreeUpdate)> {
    publish_sink: F,
    action_rx: crossbeam_channel::Receiver<ActionRequest>,
}

impl<F: FnMut(TreeUpdate)> AccessKitBridge<F> {
    /// Construct a bridge over an inbound-action receiver and an
    /// outbound-publish sink.
    pub const fn new(
        action_rx: crossbeam_channel::Receiver<ActionRequest>,
        publish_sink: F,
    ) -> Self {
        Self {
            publish_sink,
            action_rx,
        }
    }
}

impl<F: FnMut(TreeUpdate)> A11yBridge for AccessKitBridge<F> {
    type Error = BridgeError;
    type Update = TreeUpdate;
    type Action = ActionRequest;
    type NodeId = NodeId;

    fn publish(&mut self, update: Self::Update) -> Result<(), Self::Error> {
        (self.publish_sink)(update);
        Ok(())
    }

    fn take_action(&mut self) -> Option<(Self::NodeId, Self::Action)> {
        let request = self.action_rx.try_recv().ok()?;
        Some((NodeId::from_accesskit(request.target_node), request))
    }
}
