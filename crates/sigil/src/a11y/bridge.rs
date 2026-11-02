use crate::hal::A11yBridge;
use crate::kernel::NodeId;
use accesskit::{ActionRequest, TreeUpdate};

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
    type Error = crate::Error;
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
