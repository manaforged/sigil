use crate::layout::Space;
use std::rc::Rc;

use crate::kernel::{AccessibleName, NodeBuilder, Role, TextBuilder, container, text};
use crate::layout::{LayoutStyle, Overflow};
use crate::reactive::{Reactive, Signal, memo, signal};
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_string_once;
use crate::style::{BORDER_WIDTH, Paint, Style, Token, Tone};
use crate::widgets::Layout;

use super::highlight::language_for_path;
use super::layout_ext::LayoutOverrides;
use super::shared::caller_style;
use super::virtual_list::virtual_list;

mod header;
mod model;
mod patch;
mod rows;
mod selection;

pub use patch::{Hunk, Line, LineId, LineKind, Patch};
pub use selection::DiffSelection;

use model::{DiffRow, model};
use rows::ROW_INSET;
use selection::HunkPicker;

/// Diff view: side-by-side or unified rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiffView {
    /// The old text is on the left and the new text is on the right, side by side.
    Split,
    /// The removed and the added lines share one column, in patch order.
    Unified,
}

impl DiffView {
    /// The lowercase name of the view: `split` or `unified`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Split => "split",
            Self::Unified => "unified",
        }
    }
}

pub(crate) mod diff_sealed {
    use std::rc::Rc;

    use crate::reactive::{Reactive, Signal, memo, signal};

    use super::Patch;

    #[allow(unnameable_types, reason = "sealed")]
    pub trait Sealed {
        fn into_patch(self) -> Signal<Rc<Patch>>;
    }

    impl<R: Reactive<String>> Sealed for R {
        fn into_patch(self) -> Signal<Rc<Patch>> {
            memo(move || Rc::new(Patch::parse(&self.read())))
        }
    }

    impl Sealed for Patch {
        fn into_patch(self) -> Signal<Rc<Patch>> {
            signal(Rc::new(self))
        }
    }

    impl Sealed for Signal<Patch> {
        fn into_patch(self) -> Signal<Rc<Patch>> {
            memo(move || Rc::new(self.get()))
        }
    }
}

/// Input that a diff view can parse into a patch.
pub trait DiffSource: diff_sealed::Sealed {}

impl<S: diff_sealed::Sealed> DiffSource for S {}

type PatchThunk = Box<dyn FnOnce() -> Signal<Rc<Patch>>>;

#[derive(Clone)]
struct DiffShape {
    path: Option<String>,
    status: Option<String>,
    view: Signal<DiffView>,
    max_rows: usize,
    wrap: bool,
    selection: Option<Signal<DiffSelection>>,
}

/// Builder of a diff view.
#[must_use]
pub struct DiffBuilder {
    patch: PatchThunk,
    shape: DiffShape,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

impl core::fmt::Debug for DiffBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DiffBuilder")
            .field("style_accumulator", &self.style_accumulator)
            .field("layout_overrides", &self.layout_overrides)
            .finish_non_exhaustive()
    }
}

/// A rendered unified diff of text or a parsed `Patch`.
pub fn diff(patch: impl DiffSource + 'static) -> DiffBuilder {
    DiffBuilder {
        patch: Box::new(move || patch.into_patch()),
        shape: DiffShape {
            path: None,
            status: None,
            view: signal(DiffView::Split),
            max_rows: usize::MAX,
            wrap: false,
            selection: None,
        },
        style_accumulator: Style::new(),
        layout_overrides: LayoutOverrides::new(),
    }
}

impl DiffBuilder {
    /// Sets the file path shown in the header and used to pick the syntax language.
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.shape.path = Some(path.into());
        self
    }

    /// Shows `status` as a line of small text below the rows.
    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.shape.status = Some(status.into());
        self
    }

    /// Shows the rows side by side or unified; the default is `DiffView::Split`.
    pub fn view(mut self, view: impl Reactive<DiffView>) -> Self {
        self.shape.view = view.into_signal();
        self
    }

    /// Cuts the diff at `rows` rows and adds an overflow row; the default is no limit.
    pub const fn max_rows(mut self, rows: usize) -> Self {
        self.shape.max_rows = rows;
        self
    }

    /// Wraps long lines when `on` is true; the default is false.
    pub const fn wrap(mut self, on: bool) -> Self {
        self.shape.wrap = on;
        self
    }

    /// Binds the selection; a press on a hunk row or a line row updates it.
    pub const fn selected(mut self, selection: Signal<DiffSelection>) -> Self {
        self.shape.selection = Some(selection);
        self
    }

    fn root(self) -> NodeBuilder {
        let patch = (self.patch)();
        let shape = self.shape;
        let (view, max_rows) = (shape.view, shape.max_rows);
        let fallback = shape.path.as_deref().and_then(language_for_path);
        let built = memo(move || patch.with(|patch| model(patch, view.get(), fallback, max_rows)));
        let rows = memo(move || built.with(|built| Rc::clone(&built.rows)));
        let summary = memo(move || {
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
            .fit()
            .keyboard(false);
        let top = shape.clone();
        let mut root = container()
            .role(Role::List)
            .label(
                shape
                    .path
                    .clone()
                    .unwrap_or_else(|| theme_string_once(StringKey::Diff)),
            )
            .mutate(move |target| target.a11y.description = Some(AccessibleName::Derived(summary)))
            .layout(root_layout())
            .class("rounded")
            .class(Token::Field.background())
            .border_width(BORDER_WIDTH)
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
        caller_style(root, self.style_accumulator)
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
        .class(Tone::Neutral.step(11).text())
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
