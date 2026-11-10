use std::rc::Rc;

use crate::geometry::Rect;
use crate::kernel::{NodeBuilder, container};
use crate::reactive::Signal;
use crate::scene::SceneBuilder;
use crate::style::Theme;

pub(crate) type Recorder = Rc<dyn Fn(&mut SceneBuilder, Rect, &Theme)>;

#[derive(Clone)]
pub(crate) struct CanvasPaint {
    record: Recorder,
    version: Option<Signal<u64>>,
}

impl CanvasPaint {
    pub(crate) fn themed(record: Recorder) -> Self {
        Self {
            record,
            version: None,
        }
    }

    pub(crate) fn record(&self, scene: &mut SceneBuilder, rect: Rect, theme: &Theme) {
        scene.with_clip(rect, |scene| {
            let start = scene.recorded();
            (self.record)(scene, rect, theme);
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

impl core::fmt::Debug for CanvasBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CanvasBuilder")
            .field("version", &self.version)
            .field("layout_overrides", &self.layout_overrides)
            .finish_non_exhaustive()
    }
}

/// Creates a canvas widget.
#[must_use]
pub fn canvas(record: impl Fn(&mut SceneBuilder, Rect) + 'static) -> CanvasBuilder {
    CanvasBuilder {
        record: Rc::new(move |scene: &mut SceneBuilder, rect: Rect, _: &Theme| record(scene, rect)),
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
