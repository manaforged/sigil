use std::rc::Rc;

use crate::geometry::Rect;
use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, container};
use crate::reactive::{Reactive, Runtime, Signal, effect_named, signal};
use crate::scene::SceneBuilder;
use crate::scene::canvas::{CanvasPaint, Recorder};
use crate::style::Theme;
use crate::widgets::{Framed, WidgetFrame};

/// Builds a canvas that paints with a closure.
#[must_use]
pub struct CanvasBuilder {
    record: Recorder,
    redraw: Option<Box<dyn Fn()>>,
    frame: WidgetFrame,
}

impl core::fmt::Debug for CanvasBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CanvasBuilder")
            .field("redraw", &self.redraw.is_some())
            .field("frame", &self.frame)
            .finish_non_exhaustive()
    }
}

/// Creates a canvas widget whose paint callback receives the scene builder, bounds, and active theme; everything it draws is clipped to those bounds.
pub fn canvas(paint: impl Fn(&mut SceneBuilder, Rect, &Theme) + 'static) -> CanvasBuilder {
    CanvasBuilder {
        record: Rc::new(paint),
        redraw: None,
        frame: WidgetFrame::new(),
    }
}

impl CanvasBuilder {
    /// Repaints the canvas whenever a signal that `source` reads changes: a signal, an animator, or a closure.
    pub fn redraw_on<T: 'static>(mut self, source: impl Reactive<T>) -> Self {
        self.redraw = Some(Box::new(move || source.with(|_| ())));
        self
    }

    fn root(self) -> NodeBuilder {
        let (record, redraw, frame) = (self.record, self.redraw, self.frame);
        let node = container().mutate(move |node| {
            let version = redraw.map(watch);
            drop(node.attach(CanvasPaint { record, version }));
        });
        frame.finish(node)
    }
}

fn watch(read: Box<dyn Fn()>) -> Signal<u64> {
    let version = signal(0_u64);
    effect_named("sigil-canvas::redraw", move || {
        read();
        version.update(|n| *n = n.wrapping_add(1));
        Runtime::mark_frame_dirty();
    });
    version
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
