//! [`UiContext`] — host-agnostic UI driver for embedding sigil in another
//! engine's frame loop (e.g. a game engine). See DR-054.
//!
//! A standalone desktop app uses [`crate::launch`], which owns the window and
//! event loop. An embedding host owns those itself and instead drives a
//! `UiContext`: build a widget tree, lay it out at a viewport, and emit a
//! [`CommandScene`] the host paints into its own frame via the renderer's
//! `submit_screen` (which supports a foreign device).

use sigil_kernel::{Cx, NodeId};
use sigil_layout::LayoutEngine;
use sigil_layout::prelude::Size;
use sigil_renderer::{CommandScene, emit_scene};

use crate::DefaultTextMeasure;

/// A retained widget tree + layout engine an embedding host pumps each frame.
///
/// Owns no window, surface, or event loop — the host supplies the viewport
/// and paints the emitted [`CommandScene`].
pub struct UiContext {
    cx: Cx,
    layout: LayoutEngine,
    root: Option<NodeId>,
}

impl Default for UiContext {
    fn default() -> Self {
        Self::new()
    }
}

impl UiContext {
    /// Create an empty context with sigil's default (bundled-font) text
    /// measurement. Call [`UiContext::build`] before [`UiContext::scene`].
    #[must_use]
    pub fn new() -> Self {
        // Install pointer→glyph geometry alongside the measurer so embedded
        // text widgets get caret/selection hit-testing (see `text_geometry`).
        sigil_layout::set_text_geometry(std::rc::Rc::new(crate::EngineGeometry::new()));
        Self {
            cx: Cx::new_root(),
            layout: LayoutEngine::with_text_measure(Box::new(DefaultTextMeasure::new())),
            root: None,
        }
    }

    /// Build (or rebuild) the widget tree. The closure receives the context's
    /// `Cx` and returns the root node — typically `cx.add(container()…)`.
    pub fn build(&mut self, tree: impl FnOnce(&mut Cx) -> NodeId) {
        self.root = Some(tree(&mut self.cx));
    }

    /// Borrow the underlying `Cx` (for event routing / reactive updates).
    pub const fn cx(&mut self) -> &mut Cx {
        &mut self.cx
    }

    /// Lay the tree out at `viewport` (logical pixels) and emit the paint
    /// scene. Returns an empty scene until [`UiContext::build`] has run.
    pub fn scene(&mut self, viewport: (f32, f32)) -> CommandScene {
        let Some(root) = self.root else {
            return CommandScene::default();
        };
        let _ = self
            .layout
            .compute(&mut self.cx, root, Size::fixed(viewport.0, viewport.1));
        emit_scene(&self.cx, root)
    }
}
