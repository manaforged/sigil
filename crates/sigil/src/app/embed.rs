use crate::events::ShellEvent;
use crate::events::hit_test::HitTestCache;
use crate::events::{EventRouter, FocusIntent};
use crate::geometry::{Point, Rect, Size};
use crate::reactive::Runtime;
use std::rc::Rc;

use crate::kernel::{Cx, HostMode, Node, NodeId, Surface};
use crate::layout::{LayoutEngine, LayoutError, LayoutResult, TextGeometry, TextMeasure};
use crate::style::Theme;

use super::cell_text::CellGeometry;
#[cfg(all(feature = "hot", not(target_arch = "wasm32")))]
mod hot;
mod perform;
use super::host_env::{EnvGuard, HostEnv};
use crate::scene::{CommandScene, emit_scene_clipped};

#[cfg(feature = "scene")]
use crate::app::DefaultTextMeasure;

pub type AccessibilityTree = accesskit::TreeUpdate;

const LAYOUT_PASSES: usize = 4;

pub struct Host {
    cx: Cx,
    layout: LayoutEngine,
    root: Option<NodeId>,
    fill_root: bool,
    theme: Theme,
    visual_transforms: bool,
    hits: HitTestCache,
    env: HostEnv,
    #[cfg(feature = "dialog")]
    dialogs: Vec<super::dialog::layer::Mounted>,
    #[cfg(all(feature = "hot", not(target_arch = "wasm32")))]
    hot: Option<hot::HotRoot>,
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
            (0.0, geometry),
            Box::new(host_measure()),
            true,
        )
    }

    pub const CELL: f32 = 8.0;

    #[must_use]
    pub fn cells() -> Self {
        Self::with_env(
            HostMode::Cell,
            (Self::CELL, Rc::new(CellGeometry::new(Self::CELL))),
            Box::new(CellGeometry::new(Self::CELL)),
            false,
        )
    }

    fn with_env(
        mode: HostMode,
        (cell, geometry): (f32, Rc<dyn TextGeometry>),
        measure: Box<dyn TextMeasure>,
        visual_transforms: bool,
    ) -> Self {
        let mut cx = Cx::new_root();
        cx.set_host_mode(mode);
        let env = HostEnv::new(cell, geometry, cx.root_owner());
        Self {
            cx,
            layout: LayoutEngine::with_text_measure(measure),
            root: None,
            fill_root: false,
            theme: Theme::default(),
            visual_transforms,
            hits: HitTestCache::default(),
            env,
            #[cfg(feature = "dialog")]
            dialogs: Vec::new(),
            #[cfg(all(feature = "hot", not(target_arch = "wasm32")))]
            hot: None,
        }
    }

    pub const fn set_caps(&mut self, caps: super::HostCaps) {
        self.env.caps = caps;
    }

    pub(crate) fn enter(&self) -> EnvGuard {
        self.env.enter()
    }

    pub(crate) fn publish_viewport(&self, viewport: (f32, f32)) {
        self.env.publish_viewport(viewport);
        self.env.publish_scale(self.cx.scale().factor());
    }

    #[must_use]
    pub const fn host_mode(&self) -> HostMode {
        self.cx.host_mode()
    }

    pub fn take_exit(&self) -> bool {
        self.env.exit.replace(false)
    }

    pub fn mount(&mut self, app: impl crate::kernel::IntoNode) {
        #[cfg(all(feature = "hot", not(target_arch = "wasm32")))]
        self.forget_hot();
        self.replace_root(|cx| cx.add(app));
    }

    #[cfg(any(feature = "desktop", feature = "terminal"))]
    pub(crate) fn mount_app<F, N>(&mut self, app: F)
    where
        F: Fn() -> N + 'static,
        N: crate::kernel::IntoNode,
    {
        self.fill_root = true;
        #[cfg(all(feature = "hot", not(target_arch = "wasm32")))]
        self.mount_hot(app);
        #[cfg(not(all(feature = "hot", not(target_arch = "wasm32"))))]
        self.mount(app);
    }

    #[cfg(all(feature = "hot", not(target_arch = "wasm32")))]
    fn forget_hot(&mut self) {
        self.hot = None;
    }

    fn replace_root(&mut self, install: impl FnOnce(&mut Cx) -> NodeId) {
        let _env = self.enter();
        let theme = self
            .root
            .and_then(|root| self.cx.get_mut(root))
            .and_then(Node::remove_extension::<Theme>);
        if let Some(root) = self.root.take() {
            self.cx.remove(root);
        }
        let root = install(&mut self.cx);
        if self.fill_root {
            super::tree_root::fill_root(&mut self.cx, root);
        }
        self.root = Some(root);
        if let Some(theme) = theme {
            self.attach_theme(theme);
        }
        self.apply_pending_theme();
        Runtime::mark_frame_dirty();
    }

    #[cfg(feature = "dialog")]
    pub(crate) fn sync_dialogs(&mut self) {
        super::dialog::layer::sync(&mut self.cx, self.root, &mut self.dialogs);
    }

    fn attach_theme(&mut self, theme: Theme) {
        if let Some(root) = self.root
            && let Some(node) = self.cx.get_mut(root)
        {
            node.attach(theme);
        }
    }

    pub(crate) fn apply_pending_theme(&mut self) {
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
        crate::reactive::run_dispatch_deferred();
    }

    pub fn apply_focus_requests(&mut self, router: &mut EventRouter) {
        let _env = self.enter();
        if self.root.is_some() {
            router.flush_focus_requests(&mut self.cx, &self.layout);
        }
    }

    pub fn repair_focus(&mut self, router: &mut EventRouter) {
        let _env = self.enter();
        if self.root.is_some() {
            router.flush_focus_requests(&mut self.cx, &self.layout);
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

    pub(crate) fn settle_layout(&mut self, viewport: (f32, f32)) -> Result<(), LayoutError> {
        for _ in 1..LAYOUT_PASSES {
            self.layout(viewport)?;
            if !self.layout.take_republished() {
                return Ok(());
            }
        }
        self.layout(viewport).map(|_| ())
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
    pub fn theme(&self) -> &Theme {
        self.root
            .and_then(|root| self.cx.get(root))
            .and_then(Node::get_extension::<Theme>)
            .unwrap_or(&self.theme)
    }

    #[must_use]
    pub fn emit(&mut self, viewport: (f32, f32)) -> CommandScene {
        let _env = self.enter();
        self.apply_pending_theme();
        self.root.map_or_else(CommandScene::default, |root| {
            emit_viewport_scene(&self.cx, root, viewport)
        })
    }

    pub fn scene(&mut self, viewport: (f32, f32)) -> Result<CommandScene, LayoutError> {
        let _env = self.enter();
        let Some(root) = self.layout(viewport)? else {
            return Ok(CommandScene::default());
        };
        Ok(emit_viewport_scene(&self.cx, root, viewport))
    }
}

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
