#![allow(
    clippy::missing_const_for_fn,
    clippy::mod_module_files,
    clippy::struct_excessive_bools
)]

mod build;
mod end;
mod keyboard;
mod measure;
mod render;
mod row;
mod scroll;
mod selection;
mod strip;
mod window;

use crate::kernel::{IntoNode, NodeBuilder};
use crate::reactive::{Signal, effect};

use std::ops::Deref;

use super::layout_ext::LayoutOverrides;
use super::scroll::ScrollAxis;
use super::shared::ViewportSizing;
use super::table::Identifiable;
pub(crate) use end::EndReached;
pub use row::RowRender;
pub use selection::{Selection, SelectionMode};

pub(crate) const DEFAULT_ROW_HEIGHT: f32 = 32.0;
pub(crate) const DEFAULT_VIEWPORT_HEIGHT: f32 = 480.0;
const DEFAULT_OVERSCAN: usize = 1;

type ItemHeightFn<T> = dyn Fn(usize, &T) -> f32 + 'static;
pub(super) type RowKey<T> = std::rc::Rc<dyn Fn(usize, &T) -> u64>;

type BoundVisibleThunk<T, R> =
    dyn FnOnce(strip::BoundStrip<T, R>, Signal<f32>) -> NodeBuilder + 'static;
type MeasureThunk<T> = dyn FnOnce(Option<RowKey<T>>, f32) -> measure::Measure + 'static;

struct BoundItems<T, R> {
    thunk: Box<BoundVisibleThunk<T, R>>,
    measure: Option<Box<MeasureThunk<T>>>,
    live_rows: Signal<usize>,
    count: std::rc::Rc<dyn Fn() -> usize>,
}

#[derive(Clone, Copy)]
struct BuildAxes {
    row_height: f32,
    viewport_height: f32,
    width: render::WidthSpec,
    scroll_y: Signal<f32>,
    scroll_x: Option<ScrollAxis>,
    focused_row: Signal<usize>,
    selection_state: Signal<Selection>,
    viewport: Signal<crate::geometry::Size>,
    tops: Option<measure::Tops>,
    keys: Option<measure::Keys>,
    content: Signal<f32>,
    max: Signal<f32>,
}

fn bound_slice<S, T>(value: &S) -> &[T]
where
    S: Deref,
    S::Target: AsRef<[T]>,
{
    (*value).as_ref()
}

/// Builder of a virtual list.
#[must_use]
pub struct VirtualListBuilder<T, R>
where
    T: 'static,
    R: RowRender<T>,
{
    items: Vec<T>,
    bound: Option<BoundItems<T, R>>,
    render: R,
    row_key: Option<RowKey<T>>,
    row_height: f32,
    viewport_height: f32,
    viewport_width: Option<f32>,
    sizing: ViewportSizing,
    overscan: usize,
    scroll_y: Option<Signal<f32>>,
    scroll_x: Option<ScrollAxis>,
    focused_row: Option<Signal<usize>>,
    selection_state: Option<Signal<Selection>>,
    selection_mode: Option<SelectionMode>,
    keyboard_enabled: bool,
    item_height: Option<Box<ItemHeightFn<T>>>,
    row_tops: Vec<f32>,
    layout_overrides: LayoutOverrides,
    initial_scroll: Option<ScrollAnchor>,
    pin_bottom: bool,
    follow: Option<Signal<usize>>,
    content_height: Option<Signal<f32>>,
    max_scroll: Option<Signal<f32>>,
    measured: bool,
    estimate: Option<f32>,
    location: &'static core::panic::Location<'static>,
    scrollbar: bool,
    end_reached: Option<end::EndReached>,
    style_accumulator: crate::style::Style,
}

/// Where a virtual list stays scrolled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScrollAnchor {
    /// The list starts at the first row, at scroll offset 0.
    Top,
    /// The list starts scrolled to the end of the content.
    Bottom,
    /// The row at this index, at the top of the viewport when the content allows.
    Index(usize),
}

impl From<usize> for ScrollAnchor {
    fn from(index: usize) -> Self {
        Self::Index(index)
    }
}

pub(crate) mod list_sealed {
    use std::ops::Deref;

    use crate::reactive::Signal;

    use super::{RowRender, VirtualListBuilder, from_vec};

    #[allow(unnameable_types, reason = "sealed")]
    pub trait Sealed<T, R: RowRender<T>> {
        fn into_list(self, render: R) -> VirtualListBuilder<T, R>;
    }

    impl<T: 'static, R: RowRender<T>> Sealed<T, R> for Vec<T> {
        fn into_list(self, render: R) -> VirtualListBuilder<T, R> {
            from_vec(self, render)
        }
    }

    impl<S, T, R> Sealed<T, R> for Signal<S>
    where
        S: Deref + Clone + 'static,
        S::Target: AsRef<[T]>,
        T: 'static,
        R: RowRender<T>,
    {
        fn into_list(self, render: R) -> VirtualListBuilder<T, R> {
            from_vec(Vec::new(), render).bind_items(self)
        }
    }
}

/// A collection that can become a virtual list.
pub trait IntoList<T, R: RowRender<T>>: list_sealed::Sealed<T, R> {}

impl<T, R: RowRender<T>, I: list_sealed::Sealed<T, R>> IntoList<T, R> for I {}

/// A list that builds only the visible rows.
#[track_caller]
pub fn virtual_list<T, R, N, I>(items: I, render: R) -> VirtualListBuilder<T, R>
where
    T: 'static,
    R: Fn(usize, &T) -> N + 'static,
    N: IntoNode + 'static,
    I: IntoList<T, R>,
{
    let mut list = items.into_list(render);
    list.location = core::panic::Location::caller();
    list
}

fn from_vec<T, R>(items: Vec<T>, render: R) -> VirtualListBuilder<T, R>
where
    T: 'static,
    R: RowRender<T>,
{
    VirtualListBuilder {
        items,
        bound: None,
        render,
        row_key: None,
        row_height: 0.0,
        viewport_height: DEFAULT_VIEWPORT_HEIGHT,
        viewport_width: None,
        sizing: ViewportSizing::Fixed,
        overscan: DEFAULT_OVERSCAN,
        scroll_y: None,
        scroll_x: None,
        focused_row: None,
        selection_state: None,
        selection_mode: None,
        keyboard_enabled: true,
        item_height: None,
        row_tops: Vec::new(),
        layout_overrides: LayoutOverrides::new(),
        initial_scroll: None,
        pin_bottom: false,
        follow: None,
        content_height: None,
        max_scroll: None,
        measured: false,
        estimate: None,
        location: core::panic::Location::caller(),
        scrollbar: false,
        end_reached: None,
        style_accumulator: crate::style::Style::new(),
    }
}

impl<T, R> VirtualListBuilder<T, R>
where
    T: 'static,
    R: RowRender<T>,
{
    /// Keys each row by the id of its item.
    pub fn identified(self) -> Self
    where
        T: Identifiable,
    {
        self.key_by(|_, item| crate::fnv::hash_of(&item.id()))
    }

    /// Sets the function that gives each row a stable key; the default key is the row index.
    pub fn key_by(mut self, f: impl Fn(usize, &T) -> u64 + 'static) -> Self {
        self.row_key = Some(std::rc::Rc::new(f));
        self
    }

    /// Sets the fixed height of each row.
    pub fn row_height(mut self, length: impl Into<crate::layout::Length>) -> Self {
        self.row_height = crate::layout::px_or_zero(length);
        self
    }

    /// Uses the measured height of each row, and the estimate for rows not yet measured.
    pub const fn measured(mut self) -> Self {
        self.measured = true;
        self
    }

    /// Sets the height assumed for a row not yet measured; the default is the row height.
    pub fn estimate(mut self, length: impl Into<crate::layout::Length>) -> Self {
        self.estimate = crate::layout::positive_px(length);
        self
    }

    /// Sets the height in px of each row from its index and item, for a list built from a `Vec`.
    pub fn item_height(mut self, f: impl Fn(usize, &T) -> f32 + 'static) -> Self {
        self.item_height = Some(Box::new(f));
        self.row_tops.clear();
        self
    }

    /// Sets the height of the list when it does not fill its parent; the default is 480.
    pub fn viewport_height(mut self, length: impl Into<crate::layout::Length>) -> Self {
        self.viewport_height = crate::layout::px_or_zero(length);
        self
    }

    /// Sets how many viewport pages of rows to build beyond each edge; the default is 1.
    pub const fn overscan(mut self, pages: usize) -> Self {
        self.overscan = pages;
        self
    }

    /// Sets the width of the list in px; without it, the list stretches.
    pub const fn viewport_width(mut self, px: f32) -> Self {
        self.viewport_width = Some(px);
        self
    }

    /// Makes the list as high as its rows, up to the viewport height.
    pub const fn fit(mut self) -> Self {
        self.sizing = ViewportSizing::Fit;
        self
    }

    /// Binds the vertical scroll offset in px to `scroll_y`.
    pub const fn scroll_y(mut self, scroll_y: Signal<f32>) -> Self {
        self.scroll_y = Some(scroll_y);
        self
    }

    /// Binds horizontal wheel scroll to the offset `scroll_x`, its limit `max`, and view `width`.
    pub const fn scroll_x(
        mut self,
        scroll_x: Signal<f32>,
        max: Signal<f32>,
        width: Signal<f32>,
    ) -> Self {
        self.scroll_x = Some(ScrollAxis {
            offset: scroll_x,
            max,
            viewport: width,
        });
        self
    }

    /// Binds the index of the focused row to `cursor` and keeps that row in view.
    pub const fn cursor(mut self, cursor: Signal<usize>) -> Self {
        self.focused_row = Some(cursor);
        if self.follow.is_none() {
            self.follow = Some(cursor);
        }
        self
    }

    /// Sets how many rows the user can select; the default is `SelectionMode::None`.
    pub const fn selection(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = Some(mode);
        self
    }

    /// Binds the selected rows to `selection`.
    pub const fn selected(mut self, selection: Signal<Selection>) -> Self {
        self.selection_state = Some(selection);
        self
    }

    /// Turns the key handling of the list on or off; the default is on.
    pub const fn keyboard(mut self, enabled: bool) -> Self {
        self.keyboard_enabled = enabled;
        self
    }

    /// Calls `f` with the lowest selected index when the selection changes.
    pub fn change(mut self, f: impl Fn(usize) + 'static) -> Self {
        let selection = self.ensure_selection_state();
        effect(move || {
            if let Some(index) = selection.get().indices().next() {
                f(index);
            }
        });
        self
    }

    /// Starts at the end and stays there as rows are added, while the view is at the end.
    pub const fn follow_end(mut self, on: bool) -> Self {
        self.pin_bottom = on;
        self
    }

    /// Sets where the list is scrolled when it is first built: a `ScrollAnchor` or a row index.
    pub fn initial_scroll(mut self, anchor: impl Into<ScrollAnchor>) -> Self {
        self.initial_scroll = Some(anchor.into());
        self
    }

    /// Keeps the row at the index in `target` in view as `target` changes.
    pub const fn follow_row(mut self, target: Signal<usize>) -> Self {
        self.follow = Some(target);
        self
    }
}

builder_impls!([T: 'static, R: RowRender<T>] VirtualListBuilder<T, R>: paint(style_accumulator), widget);

#[cfg(test)]
mod tests;
