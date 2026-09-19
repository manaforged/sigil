use crate::events::{EventRouter, FocusIntent};
#[cfg(feature = "scene")]
use crate::hal::Point;
use crate::hal::{Rect, ShellEvent, Size};
use std::cell::Cell;
use std::rc::Rc;

use crate::kernel::{Cx, ImageCrop, ImageSource, NodeId, Surface};
use crate::layout::{LayoutEngine, LayoutError, LayoutResult, TextMeasure};
use crate::style::{Rgba8, TextStyle, Theme, Tone, grid};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::cell_text::CellGeometry;
#[cfg(feature = "scene")]
use crate::renderer::{CommandScene, emit_scene_clipped};

#[cfg(feature = "scene")]
use crate::app::DefaultTextMeasure;
use crate::app::paint::{Cascade, Resolved, Walk, fill_painted};

#[derive(Debug, Clone, PartialEq)]
pub struct Painted {
    pub id: NodeId,
    pub rect: Rect,
    pub text: Option<String>,
    pub wrap: bool,
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
        {
            crate::layout::set_text_geometry(std::rc::Rc::new(crate::app::EngineGeometry::new()));
        }
        #[cfg(not(feature = "scene"))]
        crate::layout::set_text_geometry(Rc::new(CellGeometry::new(Self::CELL)));
        Self {
            cx: Cx::new_root(),
            layout: LayoutEngine::with_text_measure(Box::new(host_measure())),
            root: None,
            theme: Theme::default(),
            resolved: Vec::new(),
            root_text: Rc::new(TextStyle::default()),
            visual_transforms: true,
        }
    }

    pub const CELL: f32 = 8.0;

    #[must_use]
    pub fn cells() -> Self {
        grid(Self::CELL);
        crate::layout::set_text_geometry(Rc::new(CellGeometry::new(Self::CELL)));
        Self {
            cx: Cx::new_root(),
            layout: LayoutEngine::with_text_measure(Box::new(CellGeometry::new(Self::CELL))),
            root: None,
            theme: Theme::default(),
            resolved: Vec::new(),
            root_text: Rc::new(TextStyle::default()),
            visual_transforms: false,
        }
    }

    pub fn build(&mut self, tree: impl FnOnce(&mut Cx) -> NodeId) {
        self.mount(tree);
    }

    pub fn mount(&mut self, tree: impl crate::app::TreeRoot) {
        self.root = Some(tree.install(&mut self.cx));
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
        router.set_visual_transforms(self.visual_transforms);
        if self.root.is_some() {
            router.route(&mut self.cx, &self.layout, event);
        }
    }

    pub fn repair_focus(&mut self, router: &mut EventRouter) {
        if self.root.is_some() {
            router.focus_if_needed(&mut self.cx, &self.layout);
        }
    }

    #[must_use]
    pub fn accessibility_tree(&self, router: &EventRouter) -> AccessibilityTree {
        crate::a11y::tree(&self.cx, router.focus(), &|id| {
            self.layout.result(id).map(|result| result.rect())
        })
    }

    pub fn focus_intent(&mut self, router: &mut EventRouter, intent: FocusIntent) {
        if self.root.is_some() {
            router.focus_intent(&mut self.cx, &self.layout, intent);
        }
    }

    pub fn layout(&mut self, viewport: (f32, f32)) -> Result<Option<NodeId>, LayoutError> {
        let Some(root) = self.root else {
            return Ok(None);
        };
        self.layout
            .compute(&mut self.cx, root, Size::fixed(viewport.0, viewport.1))?;
        Ok(Some(root))
    }

    #[must_use]
    pub fn surfaces(&self) -> Vec<(u64, Rect)> {
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
        let shrank = out.len() > next;
        out.truncate(next);
        resolved.truncate(next);
        moved || shrank
    }

    #[cfg(feature = "scene")]
    pub fn scene(&mut self, viewport: (f32, f32)) -> Result<CommandScene, LayoutError> {
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

#[derive(Clone, Copy, Default)]
pub enum CellWidth {
    #[default]
    Unicode,
    Scalar {
        joiner: usize,
        nonjoiner: usize,
        keycap: usize,
    },
}

thread_local! {
    static CELL_WIDTH: Cell<CellWidth> = const { Cell::new(CellWidth::Unicode) };
}

pub fn set_cell_width(width: CellWidth) {
    CELL_WIDTH.set(width);
}

#[must_use]
pub fn cell_width(text: &str) -> usize {
    match CELL_WIDTH.get() {
        CellWidth::Unicode => UnicodeWidthStr::width(text),
        CellWidth::Scalar {
            joiner,
            nonjoiner,
            keycap,
        } => text
            .chars()
            .map(|ch| match ch {
                '\u{200d}' => joiner,
                '\u{200c}' => nonjoiner,
                '\u{20e3}' => keycap,
                _ => UnicodeWidthChar::width(ch).unwrap_or(0),
            })
            .sum(),
    }
}

#[cfg(feature = "scene")]
fn host_measure() -> impl TextMeasure {
    DefaultTextMeasure::new()
}

#[cfg(not(feature = "scene"))]
fn host_measure() -> impl TextMeasure {
    CellGeometry::new(Host::CELL)
}
