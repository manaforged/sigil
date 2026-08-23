//! [`Host`] — embed Sigil in another engine's frame loop.
//!
//! Desktop apps call `sigil::launch`, which pumps a [`Host`]. Other
//! engines own the window and pump the same type: build, route, layout.
//! GPU paint is [`Host::scene`]
//! (this crate's `scene` feature, on with `desktop`; on `sigil-ui` that
//! is `desktop` or `render`). Cell paint is [`Host::painted`]. After
//! layout, [`Host::surfaces`] lists holes the host fills. Sigil does
//! not create OS child windows.

use sigil_events::EventRouter;
#[cfg(feature = "scene")]
use sigil_hal::Point;
use sigil_hal::{LogicalSize, Rect, ShellEvent, Size};
use std::rc::Rc;

use sigil_kernel::{Cx, NodeId, Surface, TextContent};
use sigil_layout::{LayoutEngine, LayoutError, LayoutResult, TextMeasure};
use sigil_style::{Rgba8, TextStyle, Theme, Tone, grid};
use unicode_width::UnicodeWidthStr;

#[cfg(feature = "scene")]
use sigil_renderer::{CommandScene, emit_scene_clipped};

#[cfg(feature = "scene")]
use crate::DefaultTextMeasure;
use crate::paint::{Cascade, Resolved, Walk, fill_painted};

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
    /// Resolved background colour, when this node paints a surface.
    pub fill: Option<Rgba8>,
    /// Resolved text colour, when this node paints glyphs.
    pub ink: Option<Rgba8>,
    /// Font weight of this node's text. 700 and over reads as bold.
    pub weight: u16,
    /// Nothing paints inside this box.
    ///
    /// A progress fill and a chart bar are toned boxes with no content; a
    /// button surface or a panel holds a child that paints. A terminal
    /// draws the first as solid block glyphs and only tints the second.
    pub bare: bool,
    /// Box this node paints inside, from the nearest clipping ancestor.
    ///
    /// `None` when no ancestor sets `overflow: hidden`/`scroll`. A host
    /// that ignores this paints an over-long cell over its neighbour.
    pub clip: Option<Rect>,
}

/// A retained widget tree + layout engine an embedding host pumps each frame.
///
/// Owns no window, surface, or event loop. The caller supplies the
/// viewport and paints.
///
/// `signal` / `derived` / `effect` panic outside
/// `sigil::prelude::Runtime::scope`. `launch` and `run_tui` open that
/// scope and pump this type. A host that uses signals must open it.
/// A static tree does not.
///
/// # Frame phases
///
/// `launch` runs Effect → Layout → Paint. A host that writes during
/// a paint fence should do the same: `Runtime::begin_effect`,
/// [`Host::route`], `Runtime::begin_layout`, [`Host::layout`], then
/// [`Host::painted`] or [`Host::scene`]. Tests that write in Idle
/// can skip this: `Signal::set` runs effects immediately.
///
/// ```ignore
/// use sigil::prelude::*;
///
/// let mut host = Host::new();
/// host.mount(text("hello"));
/// assert!(host.layout((80.0, 24.0)).expect("layout").is_some());
/// assert!(
///     host.painted()
///         .iter()
///         .any(|node| node.text.as_deref() == Some("hello"))
/// );
/// ```
pub struct Host {
    cx: Cx,
    layout: LayoutEngine,
    root: Option<NodeId>,
    /// Theme every node resolves against unless it carries its own.
    ///
    /// Built once: a theme reads the terminal cell grid at construction, so
    /// set the grid before building a host.
    theme: Theme,
    /// Last frame's resolved style, one entry per paint slot.
    resolved: Vec<Resolved>,
    /// Typography every node starts from.
    ///
    /// Held rather than built per frame: the cascade compares it by pointer,
    /// and a fresh one each frame would miss the cache at the root and take
    /// every node below it along.
    root_text: Rc<TextStyle>,
}

/// `surface(key)` boxes that have a layout rect. GPU paint skips these.
pub(crate) fn surface_holes(cx: &Cx, root: NodeId) -> Vec<(u64, Rect)> {
    let mut out = Vec::new();
    cx.visit_preorder(root, |_, node| {
        let Some(hole) = node.get_extension::<Surface>() else {
            return;
        };
        let Some(layout) = node.get_extension::<LayoutResult>() else {
            return;
        };
        out.push((hole.key, layout.rect()));
    });
    out
}

impl Default for Host {
    fn default() -> Self {
        Self::new()
    }
}

impl Host {
    /// Create an empty host. Call [`Host::build`] before [`Host::layout`].
    ///
    /// With this crate's `scene` feature (on with `desktop`; on
    /// `sigil-ui` that is `desktop` or `render`), text uses the
    /// production engine. Without it, text uses [`Host::CELL`].
    #[must_use]
    pub fn new() -> Self {
        #[cfg(feature = "scene")]
        {
            sigil_layout::set_text_geometry(std::rc::Rc::new(crate::EngineGeometry::new()));
        }
        Self {
            cx: Cx::new_root(),
            layout: LayoutEngine::with_text_measure(Box::new(host_measure())),
            root: None,
            theme: Theme::default(),
            resolved: Vec::new(),
            root_text: Rc::new(TextStyle::default()),
        }
    }

    /// Host for a cell grid. Layout stays in logical px; one cell is
    /// [`Self::CELL`] px so desktop widget chrome (36px buttons) maps
    /// to a few rows instead of one row per pixel.
    pub const CELL: f32 = 8.0;

    /// Empty host whose text measure matches [`Self::CELL`].
    #[must_use]
    pub fn cells() -> Self {
        // Chrome is measured in cells from here on. Set before the theme
        // below captures its metrics.
        grid(Self::CELL);
        Self {
            cx: Cx::new_root(),
            layout: LayoutEngine::with_text_measure(Box::new(CellMeasure { px: Self::CELL })),
            root: None,
            theme: Theme::default(),
            resolved: Vec::new(),
            root_text: Rc::new(TextStyle::default()),
        }
    }

    /// Build (or rebuild) the widget tree from a closure.
    ///
    /// `cx` is inferred. Same install as [`Host::mount`].
    pub fn build(&mut self, tree: impl FnOnce(&mut Cx) -> NodeId) {
        self.mount(tree);
    }

    /// Install a [`crate::TreeRoot`] — the same tree `launch` and
    /// `run_tui` accept. Closures, `text`, `image`, `container`, or
    /// [`crate::root`] wrapping a widget.
    pub fn mount(&mut self, tree: impl crate::TreeRoot) {
        self.root = Some(tree.install(&mut self.cx));
    }

    /// Borrow the underlying `Cx` (for event routing / reactive updates).
    pub const fn cx(&mut self) -> &mut Cx {
        &mut self.cx
    }

    /// Root after [`Host::build`] or [`Host::mount`].
    #[must_use]
    pub const fn root(&self) -> Option<NodeId> {
        self.root
    }

    /// `Cx`, layout engine, and root together. `launch` uses this so it
    /// does not own a second tree.
    #[cfg(feature = "desktop")]
    pub(crate) fn tree(&mut self) -> Option<(&mut Cx, &LayoutEngine, NodeId)> {
        let root = self.root?;
        Some((&mut self.cx, &self.layout, root))
    }

    /// Route a shell event into the built tree. The caller owns the
    /// [`EventRouter`] (hover and focus live there). This host supplies
    /// the `Cx` and `LayoutEngine`. No-op until [`Host::build`] has run.
    pub fn route(&mut self, router: &mut EventRouter, event: ShellEvent) {
        if self.root.is_some() {
            router.route(&mut self.cx, &self.layout, event);
        }
    }

    /// Restore focus after layout if the focused node was replaced or a
    /// newly-active scope excludes it.
    pub fn repair_focus(&mut self, router: &mut EventRouter) {
        if self.root.is_some() {
            router.focus_if_needed(&mut self.cx, &self.layout);
        }
    }

    /// Lay the tree out at `viewport`. Returns the root when a tree is built.
    ///
    /// # Errors
    ///
    /// Returns the layout engine error instead of exposing stale results.
    pub fn layout(&mut self, viewport: (f32, f32)) -> Result<Option<NodeId>, LayoutError> {
        let Some(root) = self.root else {
            return Ok(None);
        };
        self.layout
            .compute(&mut self.cx, root, Size::fixed(viewport.0, viewport.1))?;
        Ok(Some(root))
    }

    /// Native-view holes after the last [`Host::layout`].
    ///
    /// Each pair is the `surface(key)` key and its root-space rect.
    /// The GPU and TUI skip that box. The caller places a native view
    /// or ignores the list. Empty until layout.
    #[must_use]
    pub fn surfaces(&self) -> Vec<(u64, Rect)> {
        let Some(root) = self.root else {
            return Vec::new();
        };
        surface_holes(&self.cx, root)
    }

    /// Walk the last layout: rects, text, and tone. Empty until
    /// [`Host::layout`] has run.
    #[must_use]
    pub fn painted(&mut self) -> Vec<Painted> {
        let mut out = Vec::new();
        self.paint_into(&mut out);
        out
    }

    /// Emit the paint scene into `out`, reusing its allocation.
    ///
    /// [`Self::painted`] allocates a fresh vector each call. A host painting
    /// every frame keeps one buffer and calls this instead.
    pub fn paint_into(&mut self, out: &mut Vec<Painted>) {
        let _moved = self.repaint(out);
    }

    /// Emit the paint scene into `out`, reporting whether anything moved.
    ///
    /// `false` means every record matches the frame before, so a host that
    /// already drew that frame has nothing to draw or send.
    pub fn repaint(&mut self, out: &mut Vec<Painted>) -> bool {
        let Self {
            cx,
            root,
            theme,
            resolved,
            root_text,
            ..
        } = self;
        let Some(root) = *root else {
            let had = !out.is_empty();
            out.clear();
            return had;
        };
        let mut next = 0;
        let mut moved = false;
        let mut walk = Walk {
            cx,
            cache: resolved,
            out,
            next: &mut next,
            moved: &mut moved,
        };
        fill_painted(&mut walk, root, &Cascade::root(theme, root_text));
        // A record dropping off the end is a change too.
        let shrank = out.len() > next;
        out.truncate(next);
        resolved.truncate(next);
        moved || shrank
    }

    /// Lay the tree out at `viewport` (logical pixels) and emit the GPU
    /// command scene. Returns an empty scene until [`Host::build`] has
    /// run. On `sigil-ui`, enable `desktop` or `render`.
    ///
    /// # Errors
    ///
    /// Returns the layout engine error instead of emitting stale geometry.
    #[cfg(feature = "scene")]
    pub fn scene(&mut self, viewport: (f32, f32)) -> Result<CommandScene, LayoutError> {
        let Some(root) = self.layout(viewport)? else {
            return Ok(CommandScene::default());
        };
        Ok(emit_viewport_scene(&self.cx, root, viewport))
    }
}

/// GPU command scene for a laid-out tree. Launch replay and
/// [`Host::scene`] share this emit.
#[cfg(feature = "scene")]
pub(crate) fn emit_viewport_scene(cx: &Cx, root: NodeId, viewport: (f32, f32)) -> CommandScene {
    emit_scene_clipped(
        cx,
        root,
        Some(Rect::new(Point::new(0.0, 0.0), viewport.0, viewport.1)),
    )
}

/// Walk the tree, writing each node into the next slot of `out`.
///
/// Slots are overwritten rather than cleared so a node's text buffer survives
/// between frames. The tree shape is stable frame to frame, so slot `n` is
/// almost always the same node it held last time.
struct CellMeasure {
    px: f32,
}

impl TextMeasure for CellMeasure {
    #[allow(clippy::cast_precision_loss)]
    fn measure(&self, content: &TextContent, _space: Size) -> LogicalSize {
        // Display width, not char count: a CJK glyph occupies two cells, so
        // measuring by char would under-size the box that holds it.
        let cells = UnicodeWidthStr::width(content.resolve().as_str()) as f32;
        LogicalSize::new(cells * self.px, self.px)
    }
}

#[cfg(feature = "scene")]
fn host_measure() -> impl TextMeasure {
    DefaultTextMeasure::new()
}

#[cfg(not(feature = "scene"))]
fn host_measure() -> impl TextMeasure {
    CellMeasure { px: Host::CELL }
}
