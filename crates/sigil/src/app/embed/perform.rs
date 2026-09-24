use accesskit::ActionRequest;

use crate::events::EventRouter;
use crate::kernel::NodeId;

use super::Host;

impl Host {
    pub fn perform(&mut self, router: &mut EventRouter, target: NodeId, request: &ActionRequest) {
        let _env = self.enter();
        if self.root.is_some() {
            crate::a11y::actions::apply(&mut self.cx, router, &self.layout, target, request);
        }
    }
}
