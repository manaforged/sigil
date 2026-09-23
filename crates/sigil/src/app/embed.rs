use crate::events::hit_test::HitTestCache;
use crate::events::{EventRouter, FocusIntent};
use crate::hal::{Point, Rect, ShellEvent, Size};
use crate::reactive::Runtime;
use std::cell::Cell;
use std::rc::Rc;

use crate::kernel::{Cx, HostMode, ImageCrop, ImageSource, Node, NodeId, Surface};
use crate::layout::{LayoutEngine, LayoutError, LayoutResult, TextGeometry, TextMeasure};
use crate::style::{Rgba8, TextStyle, Theme, Tone};

use super::cell_text::CellGeometry;
#[cfg(feature = "scene")]
use crate::renderer::{CommandScene, emit_scene_clipped};

#[cfg(feature = "scene")]
use crate::app::DefaultTextMeasure;
use crate::app::paint::{Cascade, Resolved, TextFlow, Walk, fill_painted};

#[derive(Debug, Clone, PartialEq)]
pub struct Painted {
    pub id: NodeId,
    pub rect: Rect,
    pub text: Option<String>,
    pub flow: TextFlow,
    pub tone: Option<Tone>,
    pub fill: Option<Rgba8>,
    pub ink: Option<Rgba8>,
    pub border_widths: [f32; 4],
    pub border_color: Option<Rgba8>,
    pub border_radius: f32,
    pub weight: u16,
    pub italic: bool,
    pub bare: bool,
    pub clip: Option<Rect>,
    pub image: Option<ImageSource>,
    pub crop: Option<ImageCrop>,
}

pub type AccessibilityTree = accesskit::TreeUpdate;

pub struct Host {
    cx: Cx,
    layout: LayoutEngine,
    root: Option<NodeId>,
    theme: Theme,
    resolved: Vec<Resolved>,
    root_text: Rc<TextStyle>,
    visual_transforms: bool,
    hits: HitTestCache,
    env: HostEnv,
}

/// Host-owned process state installed on the thread while the host runs:
/// the cell grid, the text-geometry provider, and the exit flag.
struct HostEnv {
    cell: f32,
    geometry: Rc<dyn TextGeometry>,
    exit: Rc<Cell<bool>>,
    focus_source: Rc<Cell<bool>>,
}

pub(crate) struct EnvGuard {
    cell: f32,
    geometry: Option<Rc<dyn TextGeometry>>,
    exit: Option<Rc<Cell<bool>>>,
    focus_source: Option<Rc<Cell<bool>>>,
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        crate::style::swap_cell(self.cell);
        let _ = crate::layout::text_geometry::swap_geometry(self.geometry.take());
        let _ = crate::app::exit::swap_target(self.exit.take());
        let _ = crate::events::focus::swap_focus_source(self.focus_source.take());
    }
}

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
    #[must_use]
    pub fn new() -> Self {
        #[cfg(feature = "scene")]
        let geometry: Rc<dyn TextGeometry> = Rc::new(crate::app::EngineGeometry::new());
        #[cfg(not(feature = "scene"))]
        let geometry: Rc<dyn TextGeometry> = Rc::new(CellGeometry::new(Self::CELL));
        Self::with_env(
            HostMode::Pixel,
            HostEnv {
                cell: 0.0,
                geometry,
                exit: Rc::new(Cell::new(false)),
                focus_source: Rc::new(Cell::new(true)),
            },
            Box::new(host_measure()),
            true,
        )
    }

    pub const CELL: f32 = 8.0;

    #[must_use]
    pub fn cells() -> Self {
        Self::with_env(
            HostMode::Cell,
            HostEnv {
                cell: Self::CELL,
                geometry: Rc::new(CellGeometry::new(Self::CELL)),
                exit: Rc::new(Cell::new(false)),
                focus_source: Rc::new(Cell::new(true)),
            },
            Box::new(CellGeometry::new(Self::CELL)),
            false,
        )
    }

    fn with_env(
        mode: HostMode,
        env: HostEnv,
        measure: Box<dyn TextMeasure>,
        visual_transforms: bool,
    ) -> Self {
        let mut cx = Cx::new_root();
        cx.set_host_mode(mode);
        Self {
            cx,
            layout: LayoutEngine::with_text_measure(measure),
            root: None,
            theme: Theme::default(),
            resolved: Vec::new(),
            root_text: Rc::new(TextStyle::default()),
            visual_transforms,
            hits: HitTestCache::default(),
            env,
        }
    }

    pub(crate) fn enter(&self) -> EnvGuard {
        EnvGuard {
            cell: crate::style::swap_cell(self.env.cell),
            geometry: crate::layout::text_geometry::swap_geometry(Some(Rc::clone(
                &self.env.geometry,
            ))),
            exit: crate::app::exit::swap_target(Some(Rc::clone(&self.env.exit))),
            focus_source: crate::events::focus::swap_focus_source(Some(Rc::clone(
                &self.env.focus_source,
            ))),
        }
    }

    /// The mode this host was created with (`new` = pixel, `cells` = cell).
    #[must_use]
    pub const fn host_mode(&self) -> HostMode {
        self.cx.host_mode()
    }

    /// Consume this host's exit flag, independent of the active guard.
    pub fn take_exit(&self) -> bool {
        self.env.exit.replace(false)
    }

    pub fn build(&mut self, tree: impl FnOnce(&mut Cx) -> NodeId) {
        self.mount(tree);
    }

    pub fn mount(&mut self, tree: impl crate::app::TreeRoot) {
        let _env = self.enter();
        let theme = self
            .root
            .and_then(|root| self.cx.get_mut(root))
            .and_then(Node::remove_extension::<Theme>);
        if let Some(root) = self.root.take() {
            self.cx.remove(root);
        }
        self.root = Some(tree.install(&mut self.cx));
        if let Some(theme) = theme {
            self.attach_theme(theme);
        }
        #[cfg(feature = "desktop")]
        self.apply_pending_theme();
        Runtime::mark_frame_dirty();
    }

    fn attach_theme(&mut self, theme: Theme) {
        if let Some(root) = self.root
            && let Some(node) = self.cx.get_mut(root)
        {
            node.attach(theme);
        }
    }

    #[cfg(feature = "desktop")]
    fn apply_pending_theme(&mut self) {
        if self.root.is_some()
            && let Some(theme) = super::appearance::take()
        {
            self.attach_theme(theme);
        }
    }

    pub fn hit_path(&mut self, x: f32, y: f32) -> Vec<NodeId> {
        let _env = self.enter();
        if self.root.is_none() {
            return Vec::new();
        }
        self.hits
            .hit_path(&self.cx, &self.layout, Point::new(x, y))
            .map_or_else(Vec::new, smallvec::SmallVec::into_vec)
    }

    pub const fn cx(&mut self) -> &mut Cx {
        &mut self.cx
    }

    #[must_use]
    pub const fn root(&self) -> Option<NodeId> {
        self.root
    }

    #[cfg(feature = "desktop")]
    pub(crate) fn tree(&mut self) -> Option<(&mut Cx, &LayoutEngine, NodeId)> {
        let root = self.root?;
        Some((&mut self.cx, &self.layout, root))
    }

    pub fn route(&mut self, router: &mut EventRouter, event: ShellEvent) {
        let _env = self.enter();
        router.set_visual_transforms(self.visual_transforms);
        if self.root.is_some() {
            router.route(&mut self.cx, &self.layout, event);
        }
    }

    /// Route a batch of already-decoded shell events.
    pub fn route_all(
        &mut self,
        router: &mut EventRouter,
        events: impl IntoIterator<Item = ShellEvent>,
    ) {
        let _env = self.enter();
        for event in events {
            self.route(router, event);
        }
    }

    /// Apply queued `request_focus` calls. `Host::frame` runs this at the end
    /// of the frame, after layout, so requests observe fresh geometry.
    pub fn apply_focus_requests(&mut self, router: &mut EventRouter) {
        let _env = self.enter();
        if self.root.is_some() {
            router.flush_focus_requests(&mut self.cx, &self.layout);
        }
    }

    pub fn repair_focus(&mut self, router: &mut EventRouter) {
        let _env = self.enter();
        if self.root.is_some() {
            router.focus_if_needed(&mut self.cx, &self.layout);
        }
    }

    pub fn sync_hover(&mut self, router: &mut EventRouter) {
        let _env = self.enter();
        if self.root.is_some() {
            router.reconcile_hover(&self.cx, &self.layout);
        }
    }

    #[must_use]
    pub fn accessibility_tree(&self, router: &EventRouter) -> AccessibilityTree {
        let _env = self.enter();
        crate::a11y::tree(&self.cx, router.focus(), &|id| {
            self.layout.result(id).map(|result| result.rect())
        })
    }

    pub fn focus_intent(&mut self, router: &mut EventRouter, intent: FocusIntent) {
        let _env = self.enter();
        if self.root.is_some() {
            router.focus_intent(&mut self.cx, &self.layout, intent);
        }
    }

    pub fn layout(&mut self, viewport: (f32, f32)) -> Result<Option<NodeId>, LayoutError> {
        let _env = self.enter();
        let Some(root) = self.root else {
            return Ok(None);
        };
        self.layout
            .compute(&mut self.cx, root, Size::fixed(viewport.0, viewport.1))?;
        Ok(Some(root))
    }

    #[must_use]
    pub fn surfaces(&self) -> Vec<(u64, Rect)> {
        let _env = self.enter();
        let Some(root) = self.root else {
            return Vec::new();
        };
        surface_holes(&self.cx, root)
    }

    #[must_use]
    pub fn painted(&mut self) -> Vec<Painted> {
        let mut out = Vec::new();
        self.paint_into(&mut out);
        out
    }

    pub fn paint_into(&mut self, out: &mut Vec<Painted>) {
        let _moved = self.repaint(out);
    }

    pub fn repaint(&mut self, out: &mut Vec<Painted>) -> bool {
        let _env = self.enter();
        #[cfg(feature = "desktop")]
        self.apply_pending_theme();
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
            deferred: Vec::new(),
        };
        fill_painted(&mut walk, root, &Cascade::root(theme, root_text));
        let mut deferred_at = 0;
        while let Some((id, cascade)) = walk.deferred.get(deferred_at).cloned() {
            deferred_at += 1;
            fill_painted(&mut walk, id, &cascade);
        }
        let shrank = out.len() > next;
        out.truncate(next);
        resolved.truncate(next);
        moved || shrank
    }

    #[cfg(feature = "scene")]
    pub fn scene(&mut self, viewport: (f32, f32)) -> Result<CommandScene, LayoutError> {
        let _env = self.enter();
        let Some(root) = self.layout(viewport)? else {
            return Ok(CommandScene::default());
        };
        Ok(emit_viewport_scene(&self.cx, root, viewport))
    }
}

#[cfg(feature = "scene")]
pub(crate) fn emit_viewport_scene(cx: &Cx, root: NodeId, viewport: (f32, f32)) -> CommandScene {
    emit_scene_clipped(
        cx,
        root,
        Some(Rect::new(Point::new(0.0, 0.0), viewport.0, viewport.1)),
    )
}

#[cfg(feature = "scene")]
fn host_measure() -> impl TextMeasure {
    DefaultTextMeasure::new()
}

#[cfg(not(feature = "scene"))]
fn host_measure() -> impl TextMeasure {
    CellGeometry::new(Host::CELL)
}
