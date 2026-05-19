//! `A11yBridge` impls (DR-050 §3).
//!
//! Two impls land here:
//!
//! - [`RecordingBridge`] — the test-side bridge that stores published
//!   `TreeUpdate`s in a `Vec` and serves enqueued action requests from
//!   a FIFO. Tests pull the last published update to assert.
//! - The production [`accesskit::Adapter`] wiring is shell-side and
//!   not included here — the shell crate constructs a concrete adapter
//!   per platform per DR-050 §1.

use core::fmt;
use std::collections::VecDeque;

use accesskit::{ActionRequest, TreeUpdate};
use sigil_hal::A11yBridge;
use sigil_kernel::NodeId;

/// Test-side bridge — captures every `publish` into a `Vec` and
/// serves `take_action` from a FIFO queue tests prime with
/// [`RecordingBridge::action`].
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
