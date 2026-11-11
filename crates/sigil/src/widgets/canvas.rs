use std::rc::Rc;

use crate::geometry::Rect;
use crate::kernel::{NodeBuilder, container};
use crate::reactive::Signal;
use crate::scene::SceneBuilder;
use crate::scene::canvas::{CanvasPaint, Recorder};
use crate::style::Theme;

/// Builds a canvas that paints with a closure.
#[must_use]
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
pub fn canvas(record: impl Fn(&mut SceneBuilder, Rect) + 'static) -> CanvasBuilder {
    CanvasBuilder {
        record: Rc::new(move |scene: &mut SceneBuilder, rect: Rect, _: &Theme| record(scene, rect)),
        version: None,
        layout_overrides: crate::widgets::layout_ext::LayoutOverrides::new(),
    }
}

impl CanvasBuilder {
    /// Sets the redraw on.
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
