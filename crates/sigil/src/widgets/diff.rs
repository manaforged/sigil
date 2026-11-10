use crate::layout::Space;
use std::rc::Rc;

use crate::kernel::{AccessibleName, NodeBuilder, Role, TextBuilder, container, text};
use crate::layout::{LayoutStyle, Overflow};
use crate::reactive::{Reactive, Signal, derived, signal};
use crate::style::{BORDER_WIDTH, Paint, Style, Token};
use crate::widgets::Widget;

use super::highlight::language_for_path;
use super::layout_ext::LayoutOverrides;
use super::virtual_list::virtual_list;

mod header;
mod model;
mod patch;
mod rows;
mod selection;

pub use patch::{Hunk, Line, Patch};
pub use selection::Selection;

use model::{DiffRow, model};
use rows::ROW_INSET;
use selection::HunkPicker;

/// Diff view: side-by-side or unified rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffView {
    /// The split diff view.
    Split,
    /// The unified diff view.
    Unified,
}

impl DiffView {
    /// Returns the name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Split => "split",
            Self::Unified => "unified",
        }
    }
}

/// Input that a diff view can parse into a patch.
pub trait DiffSource {
    /// Converts this diff source to patch.
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
    wrap: bool,
    on_change: Option<ViewChange>,
    selection: Option<Signal<Selection>>,
}

/// Builder of a diff view.
pub struct DiffBuilder {
    patch: PatchThunk,
    shape: DiffShape,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

/// A rendered unified diff of text or a parsed `Patch`.
#[must_use]
pub fn diff(patch: impl DiffSource + 'static) -> DiffBuilder {
    DiffBuilder {
        patch: Box::new(move || patch.into_patch()),
        shape: DiffShape {
            path: None,
            status: None,
            view: DiffView::Split,
            max_rows: usize::MAX,
            wrap: false,
            on_change: None,
            selection: None,
        },
        style_accumulator: Style::new(),
        layout_overrides: LayoutOverrides::new(),
    }
}

impl DiffBuilder {
    /// Sets the path.
    #[must_use]
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.shape.path = Some(path.into());
        self
    }

    /// Sets the status.
    #[must_use]
    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.shape.status = Some(status.into());
        self
    }

    /// Sets the view.
    #[must_use]
    pub const fn view(mut self, view: DiffView) -> Self {
        self.shape.view = view;
        self
    }

    /// Sets the max rows.
    #[must_use]
    pub const fn max_rows(mut self, rows: usize) -> Self {
        self.shape.max_rows = rows;
        self
    }

    /// Sets the wrap.
    #[must_use]
    pub const fn wrap(mut self, on: bool) -> Self {
        self.shape.wrap = on;
        self
    }

    /// Sets the change.
    #[must_use]
    pub fn change(mut self, on_change: impl Fn(DiffView) + 'static) -> Self {
        self.shape.on_change = Some(Rc::new(on_change));
        self
    }

    /// Sets the line selection.
    #[must_use]
    pub const fn select(mut self, selection: Signal<Selection>) -> Self {
        self.shape.selection = Some(selection);
        self
    }

    fn root(self) -> NodeBuilder {
        let patch = (self.patch)();
        let shape = self.shape;
        let (view, max_rows) = (shape.view, shape.max_rows);
        let fallback = shape.path.as_deref().and_then(language_for_path);
        let built = derived(move || patch.with(|patch| model(patch, view, fallback, max_rows)));
        let rows = derived(move || built.with(|built| Rc::clone(&built.rows)));
        let summary = derived(move || {
            built.with(|built| format!("{} added, {} removed", built.added, built.removed))
        });
        let picker = shape
            .selection
            .map(|selection| HunkPicker { selection, patch });
        let draw = rows::Draw {
            picker,
            wrap: shape.wrap,
        };
        let list = virtual_list(rows, move |_, row: &DiffRow| rows::render(row, draw))
            .measured()
            .estimate(18.0)
            .shrink_to_content()
            .keyboard(false);
        let top = shape.clone();
        let mut root = container()
            .role(Role::List)
            .label(shape.path.clone().unwrap_or_else(|| String::from("Diff")))
            .mutate(move |target| target.a11y.description = Some(AccessibleName::Derived(summary)))
            .layout(root_layout())
            .class("rounded")
            .class(Token::Field.bg())
            .border_w(BORDER_WIDTH)
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
                .padding_x(Space::px(ROW_INSET))
                .min_size(0.0, 0.0),
        )
        .class("text-xs")
        .class(Token::Gray11.text())
}

builder_impls!([] DiffBuilder: paint(style_accumulator), widget, node);

fn root_layout() -> LayoutStyle {
    LayoutStyle::flex_column()
        .fill_width()
        .min_size(0.0, 0.0)
        .overflow_x(Overflow::Hidden)
        .gap(Space::None)
}

#[cfg(all(test, feature = "terminal"))]
#[path = "diff_tests.rs"]
mod tests;
