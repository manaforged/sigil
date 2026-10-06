use std::rc::Rc;

use crate::geometry::Rect;
use crate::kernel::{Cx, IntoNode, NodeId, container};
use crate::layout::LayoutStyle;
use crate::reactive::Signal;
use crate::scene::SceneBuilder;

type Recorder = Rc<dyn Fn(&mut SceneBuilder, Rect)>;

#[derive(Clone)]
pub(crate) struct CanvasPaint {
    record: Recorder,
    version: Option<Signal<u64>>,
}

impl CanvasPaint {
    pub(crate) fn record(&self, scene: &mut SceneBuilder, rect: Rect) {
        scene.with_clip(rect, |scene| (self.record)(scene, rect));
    }

    pub(crate) fn version(&self) -> u64 {
        self.version.and_then(Signal::try_peek).unwrap_or_default()
    }
}

pub struct CanvasBuilder {
    record: Recorder,
    version: Option<Signal<u64>>,
    layout: Option<LayoutStyle>,
}

#[must_use]
pub fn canvas(record: impl Fn(&mut SceneBuilder, Rect) + 'static) -> CanvasBuilder {
    CanvasBuilder {
        record: Rc::new(record),
        version: None,
        layout: None,
    }
}

impl CanvasBuilder {
    #[must_use]
    pub const fn redraw_on(mut self, version: Signal<u64>) -> Self {
        self.version = Some(version);
        self
    }

    #[must_use]
    pub fn layout(mut self, style: LayoutStyle) -> Self {
        self.layout = Some(style);
        self
    }
}

impl IntoNode for CanvasBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        let id = container().build(cx);
        if let Some(node) = cx.get_mut(id) {
            if let Some(style) = self.layout {
                let _ = node.attach(style);
            }
            let _ = node.attach(CanvasPaint {
                record: self.record,
                version: self.version,
            });
        }
        id
    }
}
