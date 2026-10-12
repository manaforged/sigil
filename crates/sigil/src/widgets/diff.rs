use std::rc::Rc;

use crate::kernel::{
    AccessibleName, Cx, IntoNode, NodeBuilder, Role, TextBuilder, container, text,
};
use crate::layout::{LayoutBuilderExt, LayoutStyle, Overflow};
use crate::reactive::{Reactive, Signal, derived, signal};
use crate::style::{BORDER_WIDTH, Paint, PaintTarget, Style, StyleSlot, Token};

use super::highlight::language_for_path;
use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::virtual_list::virtual_list;

mod header;
mod model;
mod patch;
mod rows;
mod selection;

pub use patch::{Hunk, Line, LineId, LineKind, Patch};
pub use selection::Selection;

use model::{DiffRow, model};
use rows::ROW_INSET;
use selection::Picker;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffView {
    Split,
    Unified,
}

impl DiffView {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Split => "split",
            Self::Unified => "unified",
        }
    }
}

pub trait DiffSource {
    fn into_patch(self) -> Signal<Rc<Patch>>;
}

impl<R: Reactive<String>> DiffSource for R {
    fn into_patch(self) -> Signal<Rc<Patch>> {
        derived(move || Rc::new(Patch::parse(&self.read())))
    }
}

impl DiffSource for Patch {
    fn into_patch(self) -> Signal<Rc<Patch>> {
        signal(Rc::new(self))
    }
}

impl DiffSource for Signal<Patch> {
    fn into_patch(self) -> Signal<Rc<Patch>> {
        derived(move || Rc::new(self.get()))
    }
}

type ViewChange = Rc<dyn Fn(DiffView)>;
type PatchThunk = Box<dyn FnOnce() -> Signal<Rc<Patch>>>;

#[derive(Clone)]
struct DiffShape {
    path: Option<String>,
    status: Option<String>,
    view: DiffView,
    max_rows: usize,
    on_change: Option<ViewChange>,
    selection: Option<Signal<Selection>>,
}

pub struct DiffBuilder {
    patch: PatchThunk,
    shape: DiffShape,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub fn diff(patch: impl DiffSource + 'static) -> DiffBuilder {
    DiffBuilder {
        patch: Box::new(move || patch.into_patch()),
        shape: DiffShape {
            path: None,
            status: None,
            view: DiffView::Split,
            max_rows: usize::MAX,
            on_change: None,
            selection: None,
        },
        style_accumulator: Style::new(),
        layout_overrides: LayoutOverrides::new(),
    }
}

impl DiffBuilder {
    #[must_use]
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.shape.path = Some(path.into());
        self
    }

    #[must_use]
    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.shape.status = Some(status.into());
        self
    }

    #[must_use]
    pub const fn view(mut self, view: DiffView) -> Self {
        self.shape.view = view;
        self
    }

    #[must_use]
    pub const fn max_rows(mut self, rows: usize) -> Self {
        self.shape.max_rows = rows;
        self
    }

    #[must_use]
    pub fn change(mut self, on_change: impl Fn(DiffView) + 'static) -> Self {
        self.shape.on_change = Some(Rc::new(on_change));
        self
    }

    #[must_use]
    pub const fn select(mut self, selection: Signal<Selection>) -> Self {
        self.shape.selection = Some(selection);
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let patch = (self.patch)();
        let shape = self.shape;
        let (view, max_rows) = (shape.view, shape.max_rows);
        let fallback = shape.path.as_deref().and_then(language_for_path);
        let built = derived(move || patch.with(|patch| model(patch, view, fallback, max_rows)));
        let rows = derived(move || built.with(|built| Rc::clone(&built.rows)));
        let summary = derived(move || {
            built.with(|built| format!("{} added, {} removed", built.added, built.removed))
        });
        let picker = shape.selection.map(|selection| Picker { selection, patch });
        let list = virtual_list(rows, move |_, row: &DiffRow| rows::render(row, picker))
            .measured()
            .estimate(18.0)
            .shrink_to_content()
            .keyboard(false);
        let top = shape.clone();
        let mut root = container()
            .role(Role::List)
            .label(shape.path.clone().unwrap_or_else(|| String::from("Diff")))
            .mutate(move |target| target.a11y.description = Some(AccessibleName::Derived(summary)))
            .layout(self.layout_overrides.clone().apply(root_layout()))
            .class("rounded")
            .class(Token::Field.bg())
            .border_w(BORDER_WIDTH.0)
            .border_color(Token::Border)
            .child(
                container()
                    .layout(root_layout())
                    .children_reactive(move || {
                        built.with(|built| vec![header::header(&top, built.added, built.removed)])
                    }),
            )
            .child(list);
        if let Some(status) = shape.status {
            root = root.child(status_row(status));
        }
        if !self.style_accumulator.is_empty() {
            root = root.style(self.style_accumulator);
        }
        root
    }
}

fn status_row(status: String) -> TextBuilder {
    text(status)
        .nowrap()
        .layout(
            LayoutStyle::block()
                .fill_width()
                .padding_x(ROW_INSET)
                .min_size(0.0, 0.0),
        )
        .class("text-xs")
        .class(Token::Gray10.text())
}

impl PaintTarget for DiffBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl WidgetLayoutExt for DiffBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

fn root_layout() -> LayoutStyle {
    LayoutStyle::flex_column()
        .fill_width()
        .min_size(0.0, 0.0)
        .overflow_x(Overflow::Hidden)
        .gap(0.0)
}

impl IntoNode for DiffBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.layout_overrides
            .hooks()
            .finish(self.into_node_builder())
            .build(cx)
    }
}

#[cfg(all(test, feature = "terminal"))]
#[path = "diff_tests.rs"]
mod tests;
