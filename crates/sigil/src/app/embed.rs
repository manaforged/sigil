use accesskit::ActionRequest;

use crate::events::Event;
use crate::events::hit_test::HitTestCache;
use crate::events::{EventRouter, FocusIntent};
use crate::geometry::{Point, Rect, Size};
use crate::reactive::Runtime;
use std::rc::Rc;

use crate::Error;
use crate::kernel::{Cx, HostMode, Node, NodeId, Surface};
use crate::layout::{LayoutEngine, LayoutResult, TextMeasure};
use crate::style::Theme;

use super::cell_text::CellGeometry;
mod access;
#[cfg(any(test, feature = "kittest", feature = "terminal"))]
mod emit;
#[cfg(all(feature = "hot", not(target_arch = "wasm32")))]
mod hot;
use super::host_env::{EnvGuard, HostEnv};
#[cfg(any(test, feature = "kittest", feature = "terminal"))]
use crate::scene::emit::EmitMemo;
use crate::scene::{CommandScene, emit_scene};

type AccessibilityTree = accesskit::TreeUpdate;

const LAYOUT_PASSES: usize = 4;

/// Embed host: build, route, lay out, and paint one tree.
pub struct Host {
    cx: Cx,
    layout: LayoutEngine,
    router: EventRouter,
    root: Option<NodeId>,
    fill_root: bool,
    theme: Theme,
    visual_transforms: bool,
    hits: HitTestCache,
    #[cfg(any(test, feature = "kittest", feature = "terminal"))]
    emitted: EmitMemo,
    env: HostEnv,
    #[cfg(feature = "dialog")]
    dialogs: Vec<super::dialog::layer::Mounted>,
    #[cfg(all(feature = "hot", not(target_arch = "wasm32")))]
    hot: Option<hot::HotRoot>,
}

pub(crate) fn surface_holes(cx: &Cx, root: NodeId) -> Vec<(u64, Rect)> {
    let mut out = Vec::new();
    cx.visit_preorder(root, |_, node| {
        if let (Some(hole), Some(layout)) = (
            node.get_extension::<Surface>(),
            node.get_extension::<LayoutResult>(),
        ) {
            out.push((hole.key, layout.rect()));
        }
    });
    out
}

impl Host {
    /// Creates a host for pixel or cell layout.
    #[must_use]
    pub fn new(mode: HostMode) -> Self {
        match mode {
            HostMode::Pixel => Self::pixels(),
            HostMode::Cell => Self::with_env(
                HostMode::Cell,
                (Self::CELL, Rc::new(CellGeometry::new(Self::CELL))),
                Box::new(CellGeometry::new(Self::CELL)),
                false,
            ),
        }
    }

    fn pixels() -> Self {
        #[cfg(feature = "scene")]
        let (geometry, measure): (Rc<dyn TextMeasure>, Box<dyn TextMeasure>) = (
            Rc::new(crate::app::DefaultTextMeasure::new()),
            Box::new(crate::app::DefaultTextMeasure::new()),
        );
        #[cfg(not(feature = "scene"))]
        let (geometry, measure): (Rc<dyn TextMeasure>, Box<dyn TextMeasure>) = (
            Rc::new(CellGeometry::new(Self::CELL)),
            Box::new(CellGeometry::new(Self::CELL)),
        );
        Self::with_env(HostMode::Pixel, (0.0, geometry), measure, true)
    }

    /// The width of one terminal cell in layout units.
    pub const CELL: f32 = 8.0;

    fn with_env(
        mode: HostMode,
        (cell, geometry): (f32, Rc<dyn TextMeasure>),
        measure: Box<dyn TextMeasure>,
        visual_transforms: bool,
    ) -> Self {
        let mut cx = Cx::new_root();
        cx.set_host_mode(mode);
        let env = HostEnv::new(cell, geometry, cx.root_owner());
        Self {
            cx,
            layout: LayoutEngine::with_text_measure(measure),
            router: EventRouter::new(),
            root: None,
            fill_root: false,
            theme: Theme::default(),
            visual_transforms,
            hits: HitTestCache::default(),
            #[cfg(any(test, feature = "kittest", feature = "terminal"))]
            emitted: EmitMemo::default(),
            env,
            #[cfg(feature = "dialog")]
            dialogs: Vec::new(),
            #[cfg(all(feature = "hot", not(target_arch = "wasm32")))]
            hot: None,
        }
    }

    /// Tells widgets whether the host can show hyperlinks.
    pub const fn set_hyperlinks(&mut self, yes: bool) {
        self.env.caps.hyperlinks = yes;
    }

    pub(crate) fn enter(&self) -> EnvGuard {
        self.env.enter()
    }

    pub(crate) fn publish_viewport(&self, viewport: (f32, f32)) {
        self.env.publish_viewport(viewport);
    }

    /// Returns the host mode.
    #[must_use]
    pub const fn host_mode(&self) -> HostMode {
        self.cx.host_mode()
    }

    /// Returns whether take exit is set.
    pub fn take_exit(&self) -> bool {
        self.env.exit.replace(false)
    }

    /// Builds `app` and makes it the root of the tree.
    pub fn mount(&mut self, app: impl crate::kernel::IntoNode) {
        #[cfg(all(feature = "hot", not(target_arch = "wasm32")))]
        {
            self.hot = None;
        }
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

    pub(crate) fn attach_theme(&mut self, theme: Theme) {
        if let Some(root) = self.root
            && let Some(node) = self.cx.get_mut(root)
        {
            self.env.theme.set(theme.clone());
            node.attach(theme);
            crate::scene::emit_cache::theme_changed();
        }
    }

    pub(crate) fn apply_pending_theme(&mut self) {
        if self.root.is_some()
            && let Some(theme) = super::appearance::take()
        {
            self.attach_theme(theme);
        }
    }

    /// Returns the nodes under the point, from the deepest node up.
    pub fn hit_path(&mut self, x: f32, y: f32) -> Vec<NodeId> {
        let _env = self.enter();
        if self.root.is_none() {
            return Vec::new();
        }
        self.hits
            .hit_path(&self.cx, &self.layout, Point::new(x, y))
            .map_or_else(Vec::new, smallvec::SmallVec::into_vec)
    }

    /// Returns the node tree context.
    pub const fn cx(&mut self) -> &mut Cx {
        &mut self.cx
    }

    /// Returns the root.
    #[must_use]
    pub const fn root(&self) -> Option<NodeId> {
        self.root
    }

    #[cfg(debug_assertions)]
    pub(crate) fn report_flaws(&self) {
        use crate::layout::audit::Tolerance;
        thread_local! {
            static SEEN: std::cell::RefCell<std::collections::HashSet<String>> = std::cell::RefCell::default();
        }
        let flaws = self.flaws(Tolerance::for_mode(self.host_mode()));
        SEEN.with_borrow_mut(|seen| {
            for flaw in flaws.iter().map(ToString::to_string) {
                if seen.insert(flaw.clone()) {
                    tracing::warn!(target: "sigil::design", "{flaw}");
                    if matches!(self.host_mode(), crate::kernel::HostMode::Pixel) {
                        eprintln!("sigil design flaw: {flaw}");
                    }
                }
            }
        });
    }

    #[cfg(any(
        test,
        debug_assertions,
        feature = "kittest",
        all(feature = "inspect", any(feature = "desktop", feature = "terminal"))
    ))]
    #[must_use]
    pub(crate) fn flaws(
        &self,
        tolerance: crate::layout::audit::Tolerance,
    ) -> Vec<crate::layout::audit::Flaw> {
        let _env = self.enter();
        self.root.map_or_else(Vec::new, |root| {
            crate::layout::audit::flaws(&self.cx, &self.layout, (root, self.theme()), tolerance)
        })
    }

    #[cfg(feature = "desktop")]
    pub(crate) fn tree(&mut self) -> Option<(&mut Cx, &LayoutEngine, &mut EventRouter, NodeId)> {
        let root = self.root?;
        Some((&mut self.cx, &self.layout, &mut self.router, root))
    }

    /// Sends an input event through the tree.
    pub fn route(&mut self, event: Event) {
        let _env = self.enter();
        self.router.set_visual_transforms(self.visual_transforms);
        if self.root.is_some() {
            self.router.route(&mut self.cx, &self.layout, event);
        }
        crate::reactive::run_dispatch_deferred();
    }

    fn on_tree(&mut self, run: impl FnOnce(&mut Cx, &LayoutEngine, &mut EventRouter)) {
        let _env = self.enter();
        if self.root.is_some() {
            run(&mut self.cx, &self.layout, &mut self.router);
        }
    }

    pub(crate) fn apply_focus_requests(&mut self) {
        self.on_tree(|cx, layout, router| router.flush_focus_requests(cx, layout));
    }

    /// Moves focus off a node that can no longer hold it.
    pub fn repair_focus(&mut self) {
        self.on_tree(|cx, layout, router| {
            router.flush_focus_requests(cx, layout);
            router.focus_if_needed(cx, layout);
        });
    }

    pub(crate) fn sync_hover(&mut self) {
        self.on_tree(|cx, layout, router| router.reconcile_hover(cx, layout));
    }

    /// Runs an accessibility action on `target`.
    pub fn perform(&mut self, target: NodeId, request: &ActionRequest) {
        self.on_tree(|cx, layout, router| {
            crate::a11y::actions::apply(cx, router, layout, target, request);
        });
    }

    /// Applies a focus request.
    pub fn focus_intent(&mut self, intent: FocusIntent) {
        self.on_tree(|cx, layout, router| router.focus_intent(cx, layout, intent));
    }

    /// Returns the focus.
    #[must_use]
    pub const fn focus(&self) -> Option<NodeId> {
        self.router.focus()
    }

    #[cfg(any(test, feature = "kittest"))]
    pub const fn parts(&mut self) -> (&mut Cx, &LayoutEngine, &mut EventRouter) {
        (&mut self.cx, &self.layout, &mut self.router)
    }

    /// Lays out the tree for `viewport` and returns the root.
    pub fn layout(&mut self, viewport: (f32, f32)) -> Result<Option<NodeId>, Error> {
        let _env = self.enter();
        let Some(root) = self.root else {
            return Ok(None);
        };
        self.layout
            .compute(&mut self.cx, root, Size::fixed(viewport.0, viewport.1))?;
        Ok(Some(root))
    }

    pub(crate) fn settle_layout(&mut self, viewport: (f32, f32)) -> Result<(), Error> {
        for _ in 1..LAYOUT_PASSES {
            self.layout(viewport)?;
            if !self.layout.take_republished() {
                return Ok(());
            }
        }
        self.layout(viewport).map(|_| ())
    }

    /// Returns the surfaces.
    #[must_use]
    pub fn surfaces(&self) -> Vec<(u64, Rect)> {
        let _env = self.enter();
        let Some(root) = self.root else {
            return Vec::new();
        };
        surface_holes(&self.cx, root)
    }

    /// Returns the theme.
    #[must_use]
    pub fn theme(&self) -> &Theme {
        self.root
            .and_then(|root| self.cx.get(root))
            .and_then(Node::get_extension::<Theme>)
            .unwrap_or(&self.theme)
    }

    /// Paints the laid-out tree into a new scene.
    #[must_use]
    pub fn emit(&mut self, viewport: (f32, f32)) -> CommandScene {
        let _env = self.enter();
        self.apply_pending_theme();
        self.root.map_or_else(CommandScene::default, |root| {
            emit_viewport_scene(&self.cx, root, viewport)
        })
    }

    /// Lays out the tree for `viewport` and paints it into a new scene.
    pub fn scene(&mut self, viewport: (f32, f32)) -> Result<CommandScene, Error> {
        let _env = self.enter();
        let Some(root) = self.layout(viewport)? else {
            return Ok(CommandScene::default());
        };
        Ok(emit_viewport_scene(&self.cx, root, viewport))
    }
}

pub(crate) fn emit_viewport_scene(cx: &Cx, root: NodeId, viewport: (f32, f32)) -> CommandScene {
    emit_scene(cx, root, Some(viewport_rect(viewport)))
}

const fn viewport_rect(viewport: (f32, f32)) -> Rect {
    Rect::new(Point::new(0.0, 0.0), viewport.0, viewport.1)
}
