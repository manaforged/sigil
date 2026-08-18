//! [`Host`] — embed Sigil in another engine's frame loop.
//!
//! Desktop apps call `sigil::launch`. Other engines (a game loop, a
//! custom window, a terminal) own the surface and pump a [`Host`]:
//! build, route events, lay out. Desktop paint is [`Host::scene`].

use sigil_events::EventRouter;
#[cfg(feature = "desktop")]
use sigil_hal::Point;
use sigil_hal::{LogicalSize, Rect, ShellEvent, Size};
use sigil_kernel::{Cx, NodeId, NodeType, TextContent};
use sigil_layout::LayoutEngine;
use sigil_layout::LayoutResult;
use sigil_layout::TextMeasure;
use sigil_style::{Tone, ToneContext};

#[cfg(feature = "desktop")]
use sigil_renderer::{CommandScene, emit_scene_clipped};

#[cfg(feature = "desktop")]
use crate::DefaultTextMeasure;

/// A laid-out node after [`Host::layout`]. No GPU types.
#[derive(Debug, Clone, PartialEq)]
pub struct Painted {
    /// Source node.
    pub id: NodeId,
    /// Layout rectangle in host units.
    pub rect: Rect,
    /// Resolved text when this node is a text leaf.
    pub text: Option<String>,
    /// Tone attached to this node, if any.
    pub tone: Option<Tone>,
}

/// A retained widget tree + layout engine an embedding host pumps each frame.
///
/// Owns no window, surface, or event loop — the host supplies the viewport
/// and paints. Desktop callers use [`Host::scene`]; TUI and tests use
/// [`Host::layout`] + [`Host::painted`].
pub struct Host {
    cx: Cx,
    layout: LayoutEngine,
    root: Option<NodeId>,
}

impl Default for Host {
    fn default() -> Self {
        Self::new()
    }
}

impl Host {
    /// Create an empty host. Call [`Host::build`] before [`Host::layout`].
    #[must_use]
    pub fn new() -> Self {
        #[cfg(feature = "desktop")]
        {
            // Install pointer→glyph geometry alongside the measurer so embedded
            // text widgets get caret/selection hit-testing (see `text_geometry`).
            sigil_layout::set_text_geometry(std::rc::Rc::new(crate::EngineGeometry::new()));
        }
        Self {
            cx: Cx::new_root(),
            layout: LayoutEngine::with_text_measure(Box::new(host_measure())),
            root: None,
        }
    }

    /// Host for a cell grid. Layout stays in logical px; one cell is
    /// [`Self::CELL`] px so desktop widget chrome (36px buttons) maps
    /// to a few rows instead of one row per pixel.
    pub const CELL: f32 = 8.0;

    /// Empty host whose text measure matches [`Self::CELL`].
    #[must_use]
    pub fn cells() -> Self {
        Self {
            cx: Cx::new_root(),
            layout: LayoutEngine::with_text_measure(Box::new(CellMeasure { px: Self::CELL })),
            root: None,
        }
    }

    /// Build (or rebuild) the widget tree. The closure receives this
    /// host's `Cx` and returns the root node.
    pub fn build(&mut self, tree: impl FnOnce(&mut Cx) -> NodeId) {
        self.root = Some(tree(&mut self.cx));
    }

    /// Install a [`crate::TreeRoot`] — the same tree `launch` and
    /// `run_tui` accept.
    pub fn mount(&mut self, tree: impl crate::TreeRoot) {
        self.root = Some(tree.install(&mut self.cx));
    }

    /// Borrow the underlying `Cx` (for event routing / reactive updates).
    pub const fn cx(&mut self) -> &mut Cx {
        &mut self.cx
    }

    /// Route a shell event into the built tree. The caller owns the
    /// [`EventRouter`] (hover and focus live there). This host supplies
    /// the `Cx` and `LayoutEngine`. No-op until [`Host::build`] has run.
    pub fn route(&mut self, router: &mut EventRouter, event: ShellEvent) {
        if self.root.is_some() {
            router.route(&mut self.cx, &self.layout, event);
        }
    }

    /// Lay the tree out at `viewport`. Returns the root when a tree is built.
    pub fn layout(&mut self, viewport: (f32, f32)) -> Option<NodeId> {
        let root = self.root?;
        let _ = self
            .layout
            .compute(&mut self.cx, root, Size::fixed(viewport.0, viewport.1));
        Some(root)
    }

    /// Walk the last layout: rects, text, and tone. Empty until
    /// [`Host::layout`] has run.
    #[must_use]
    pub fn painted(&self) -> Vec<Painted> {
        let Some(root) = self.root else {
            return Vec::new();
        };
        let mut out = Vec::new();
        push_painted(&self.cx, root, &mut out);
        out
    }

    /// Lay the tree out at `viewport` (logical pixels) and emit the paint
    /// scene. Returns an empty scene until [`Host::build`] has run.
    #[cfg(feature = "desktop")]
    pub fn scene(&mut self, viewport: (f32, f32)) -> CommandScene {
        let Some(root) = self.layout(viewport) else {
            return CommandScene::default();
        };
        emit_scene_clipped(
            &self.cx,
            root,
            Some(Rect::new(Point::new(0.0, 0.0), viewport.0, viewport.1)),
        )
    }
}

fn push_painted(cx: &Cx, id: NodeId, out: &mut Vec<Painted>) {
    let Some(node) = cx.get(id) else {
        return;
    };
    if let Some(result) = node.get_extension::<LayoutResult>() {
        let text = match &node.kind {
            NodeType::Text { content, .. } => Some(content.resolve()),
            NodeType::Container | NodeType::Image { .. } | NodeType::Fragment => None,
        };
        out.push(Painted {
            id,
            rect: result.rect(),
            text,
            tone: node.get_extension::<ToneContext>().map(|ctx| ctx.tone),
        });
    }
    for child in node.children() {
        push_painted(cx, *child, out);
    }
}

struct CellMeasure {
    px: f32,
}

impl TextMeasure for CellMeasure {
    #[allow(clippy::cast_precision_loss)]
    fn measure(&self, content: &TextContent, _space: Size) -> LogicalSize {
        let n = content.resolve().chars().count() as f32;
        LogicalSize::new(n * self.px, self.px)
    }
}

#[cfg(feature = "desktop")]
fn host_measure() -> impl TextMeasure {
    DefaultTextMeasure::new()
}

#[cfg(not(feature = "desktop"))]
fn host_measure() -> impl TextMeasure {
    CellMeasure { px: 1.0 }
}
