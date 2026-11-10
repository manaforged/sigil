use std::rc::Rc;
use std::sync::mpsc::Receiver;

use crate::kernel::NodeId;
use accesskit::{ActionRequest, TreeUpdate};

pub struct AccessKitBridge<F: FnMut(TreeUpdate)> {
    publish_sink: F,
    action_rx: Rc<Receiver<ActionRequest>>,
}

impl<F: FnMut(TreeUpdate)> core::fmt::Debug for AccessKitBridge<F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AccessKitBridge")
            .field("action_rx", &self.action_rx)
            .finish_non_exhaustive()
    }
}

impl<F: FnMut(TreeUpdate)> AccessKitBridge<F> {
    pub const fn new(action_rx: Rc<Receiver<ActionRequest>>, publish_sink: F) -> Self {
        Self {
            publish_sink,
            action_rx,
        }
    }

    /// Sends a tree update to the platform accessibility adapter.
    pub fn publish(&mut self, update: TreeUpdate) {
        (self.publish_sink)(update);
    }

    /// Takes the next action request from assistive technology, if one waits.
    pub fn take_action(&self) -> Option<(NodeId, ActionRequest)> {
        let request = self.action_rx.try_recv().ok()?;
        Some((NodeId::from_accesskit(request.target_node), request))
    }
}
