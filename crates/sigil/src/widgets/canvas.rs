use std::rc::Rc;

use crate::geometry::Rect;
use crate::kernel::{NodeBuilder, container};
use crate::reactive::Signal;
use crate::scene::SceneBuilder;
use crate::scene::canvas::{CanvasPaint, Recorder};
use crate::style::{Style, Theme};

use super::shared::caller_style;

/// Builds a canvas that paints with a closure.
#[must_use]
pub struct CanvasBuilder {
    record: Recorder,
    version: Option<Signal<u64>>,
    style_accumulator: Style,
    layout_overrides: crate::widgets::layout_ext::LayoutOverrides,
}

impl core::fmt::Debug for CanvasBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CanvasBuilder")
            .field("version", &self.version)
            .field("style_accumulator", &self.style_accumulator)
            .field("layout_overrides", &self.layout_overrides)
            .finish_non_exhaustive()
    }
}

/// Creates a canvas widget.
pub fn canvas(record: impl Fn(&mut SceneBuilder, Rect) + 'static) -> CanvasBuilder {
    CanvasBuilder {
        record: Rc::new(move |scene: &mut SceneBuilder, rect: Rect, _: &Theme| record(scene, rect)),
        version: None,
        style_accumulator: Style::new(),
        layout_overrides: crate::widgets::layout_ext::LayoutOverrides::new(),
    }
}

impl CanvasBuilder {
    /// Repaints the canvas whenever version changes.
    pub const fn redraw_on(mut self, version: Signal<u64>) -> Self {
        self.version = Some(version);
        self
    }

    /// Replaces the paint callback with one that also receives the active theme.
    pub fn themed(mut self, record: impl Fn(&mut SceneBuilder, Rect, &Theme) + 'static) -> Self {
        self.record = Rc::new(record);
        self
    }

    fn root(self) -> NodeBuilder {
        let (record, version) = (self.record, self.version);
        let node = container().mutate(move |node| {
            drop(node.attach(CanvasPaint { record, version }));
        });
        caller_style(node, self.style_accumulator)
    }
}

builder_impls!([] CanvasBuilder: paint(style_accumulator), widget, node);
