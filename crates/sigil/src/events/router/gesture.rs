use crate::events::pointer_dispatch::{local_point, pointer_targets};
use crate::hal::events::GestureEvent;
use crate::kernel::{Cx, NodeBuilder};
use crate::layout::LayoutEngine;
use crate::reactive::owner_by_id;

use super::EventRouter;

type GestureHandler = Box<dyn Fn(&mut GestureEvent) + 'static>;

#[derive(Default)]
pub(crate) struct GestureHandlers {
    handlers: Vec<GestureHandler>,
}

impl GestureHandlers {
    fn invoke(&self, event: &mut GestureEvent) {
        for handler in &self.handlers {
            handler(event);
            if event.propagation_stopped() {
                return;
            }
        }
    }
}

impl core::fmt::Debug for GestureHandlers {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("GestureHandlers").finish_non_exhaustive()
    }
}

pub(crate) fn on_gesture(
    builder: NodeBuilder,
    handler: impl Fn(&mut GestureEvent) + 'static,
) -> NodeBuilder {
    builder.mutate(move |node| {
        if node.get_extension::<GestureHandlers>().is_none() {
            let _ = node.attach(GestureHandlers::default());
        }
        if let Some(handlers) = node.get_extension_mut::<GestureHandlers>() {
            handlers.handlers.push(Box::new(handler));
        }
    })
}

impl EventRouter {
    pub(super) fn route_gesture(
        &mut self,
        cx: &Cx,
        layout: &LayoutEngine,
        mut event: GestureEvent,
    ) {
        let Some(path) = self.hit_cache.hit_path(cx, layout, event.position) else {
            return;
        };
        let global = event.position;
        let targets = pointer_targets(cx, layout, &path, self.hit_cache.uses_transforms());
        for target in targets.iter().rev().copied() {
            let Some(node) = cx.get(target.id) else {
                continue;
            };
            if let Some(handlers) = node.get_extension::<GestureHandlers>() {
                event.position = local_point(cx, target, global);
                let _ = owner_by_id(node.owner, || handlers.invoke(&mut event));
            }
            if event.propagation_stopped() {
                return;
            }
        }
    }
}
