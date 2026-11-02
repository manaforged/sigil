use std::rc::Rc;

use crate::geometry::Rect;
use crate::kernel::{NodeBuilder, container};
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
        scene.with_clip(rect, |scene| {
            let start = scene.recorded();
            (self.record)(scene, rect);
            scene.mark_leaf_from(start);
        });
    }

    pub(crate) fn version(&self) -> u64 {
        self.version.and_then(Signal::try_peek).unwrap_or_default()
    }
}

/// Builds a canvas that paints with a closure.
pub struct CanvasBuilder {
    record: Recorder,
    version: Option<Signal<u64>>,
    layout_overrides: crate::widgets::layout_ext::LayoutOverrides,
}

/// Creates a canvas widget.
#[must_use]
pub fn canvas(record: impl Fn(&mut SceneBuilder, Rect) + 'static) -> CanvasBuilder {
    CanvasBuilder {
        record: Rc::new(record),
        version: None,
        layout_overrides: crate::widgets::layout_ext::LayoutOverrides::new(),
    }
}

impl CanvasBuilder {
    /// Sets the redraw on.
    #[must_use]
    pub const fn redraw_on(mut self, version: Signal<u64>) -> Self {
        self.version = Some(version);
        self
    }

    fn root(self) -> NodeBuilder {
        let (record, version) = (self.record, self.version);
        container().mutate(move |node| {
            let _ = node.attach(CanvasPaint { record, version });
        })
    }
}

builder_impls!([] CanvasBuilder: widget, node);
