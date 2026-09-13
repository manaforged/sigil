use core::fmt;
use std::collections::VecDeque;

use crate::hal::A11yBridge;
use crate::kernel::NodeId;
use accesskit::{ActionRequest, TreeUpdate};

#[doc(hidden)]
#[derive(Debug, Default)]
pub struct RecordingBridge {
    pub(crate) published: Vec<TreeUpdate>,
    pub pending_actions: VecDeque<(NodeId, ActionRequest)>,
}

impl RecordingBridge {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn action(&mut self, target: NodeId, request: ActionRequest) {
        self.pending_actions.push_back((target, request));
    }

    #[must_use]
    pub fn last_published(&self) -> Option<&TreeUpdate> {
        self.published.last()
    }

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

#[derive(Debug)]
pub struct BridgeError;

impl fmt::Display for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("accesskit bridge rejected update")
    }
}

impl core::error::Error for BridgeError {}

pub struct AccessKitBridge<F: FnMut(TreeUpdate)> {
    publish_sink: F,
    action_rx: crossbeam_channel::Receiver<ActionRequest>,
}

impl<F: FnMut(TreeUpdate)> AccessKitBridge<F> {
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
