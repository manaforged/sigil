use std::rc::Rc;

use crate::geometry::Rect;
use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, container};
use crate::reactive::Signal;
use crate::scene::SceneBuilder;
use crate::scene::canvas::{CanvasPaint, Recorder};
use crate::style::Theme;
use crate::widgets::{Framed, WidgetFrame};

/// Builds a canvas that paints with a closure.
#[must_use]
pub struct CanvasBuilder {
    record: Recorder,
    version: Option<Signal<u64>>,
    frame: WidgetFrame,
}

impl core::fmt::Debug for CanvasBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CanvasBuilder")
            .field("version", &self.version)
            .field("frame", &self.frame)
            .finish_non_exhaustive()
    }
}

/// Creates a canvas widget whose paint callback receives the scene builder, bounds, and active theme.
pub fn canvas(paint: impl Fn(&mut SceneBuilder, Rect, &Theme) + 'static) -> CanvasBuilder {
    CanvasBuilder {
        record: Rc::new(paint),
        version: None,
        frame: WidgetFrame::new(),
    }
}

impl CanvasBuilder {
    /// Repaints the canvas whenever `version` changes.
    pub const fn redraw_on(mut self, version: Signal<u64>) -> Self {
        self.version = Some(version);
        self
    }

    fn root(self) -> NodeBuilder {
        let (record, version, frame) = (self.record, self.version, self.frame);
        let node = container().mutate(move |node| {
            drop(node.attach(CanvasPaint { record, version }));
        });
        frame.finish(node)
    }
}

impl Framed for CanvasBuilder {
    fn frame_mut(&mut self) -> &mut WidgetFrame {
        &mut self.frame
    }
}

impl IntoNode for CanvasBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.root().build(cx)
    }
}
