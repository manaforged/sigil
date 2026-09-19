use crate::hal::Size;
use crate::kernel::{Cx, NodeId};
use crate::layout::LayoutError;
use crate::style::StyleBinding;

use super::LayoutEngine;
use super::measure::to_available_space;

#[derive(Clone, Copy)]
pub(crate) struct ViewportOverlay;

impl LayoutEngine {
    pub(super) fn measure_viewport_overlays(
        &mut self,
        cx: &mut Cx,
        root: NodeId,
        available: Size,
    ) -> Result<(), LayoutError> {
        let mut overlays = Vec::new();
        collect(cx, root, &mut overlays);
        for id in overlays {
            self.drive
                .measure_keep_frame(self.handle(id)?, to_available_space(available))?;
            self.attach_subtree_results(cx, id)?;
        }
        Ok(())
    }
}

fn collect(cx: &Cx, id: NodeId, overlays: &mut Vec<NodeId>) {
    let Some(node) = cx.get(id) else {
        return;
    };
    if node
        .get_extension::<StyleBinding>()
        .is_some_and(StyleBinding::hides)
    {
        return;
    }
    if node.get_extension::<ViewportOverlay>().is_some() {
        overlays.push(id);
    }
    for child in node.children() {
        collect(cx, *child, overlays);
    }
}
