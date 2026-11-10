#![allow(
    clippy::missing_const_for_fn,
    clippy::mod_module_files,
    clippy::struct_excessive_bools
)]

mod build;
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
use super::table::Identifiable;
pub use row::RowRender;
pub use selection::{SelectionMode, SelectionState};

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
    selection_state: Signal<SelectionState>,
    viewport: Signal<(f32, f32)>,
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
    sizing: ListSizing,
    overscan: usize,
    scroll_y: Option<Signal<f32>>,
    scroll_x: Option<ScrollAxis>,
    focused_row: Option<Signal<usize>>,
    selection_state: Option<Signal<SelectionState>>,
    selection_mode: SelectionMode,
    keyboard_enabled: bool,
    item_height: Option<Box<ItemHeightFn<T>>>,
    row_tops: Vec<f32>,
    layout_overrides: LayoutOverrides,
    initial_scroll: Option<ScrollAnchor>,
    pin_bottom: bool,
    follow: Option<Signal<usize>>,
    content_height_bind: Option<Signal<f32>>,
    max_scroll_bind: Option<Signal<f32>>,
    measured: bool,
    estimate: Option<f32>,
    location: &'static core::panic::Location<'static>,
    scrollbar: bool,
    style_accumulator: crate::style::Style,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ListSizing {
    Fixed,
    Fill,
    ShrinkToContent,
}

/// Where a virtual list stays scrolled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollAnchor {
    /// The top scroll anchor.
    Top,
    /// The bottom scroll anchor.
    Bottom,
    /// The index scroll anchor.
    Index(usize),
}

/// A collection that can become a virtual list.
pub trait IntoList<T, R: RowRender<T>> {
    /// Creates a virtual list that renders with `render`.
    fn into_list(self, render: R) -> VirtualListBuilder<T, R>;
}

impl<T: 'static, R: RowRender<T>> IntoList<T, R> for Vec<T> {
    fn into_list(self, render: R) -> VirtualListBuilder<T, R> {
        from_vec(self, render)
    }
}

impl<S, T, R> IntoList<T, R> for Signal<S>
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

/// A list that builds only the visible rows.
#[must_use]
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
        sizing: ListSizing::Fixed,
        overscan: DEFAULT_OVERSCAN,
        scroll_y: None,
        scroll_x: None,
        focused_row: None,
        selection_state: None,
        selection_mode: SelectionMode::None,
        keyboard_enabled: true,
        item_height: None,
        row_tops: Vec::new(),
        layout_overrides: LayoutOverrides::new(),
        initial_scroll: None,
        pin_bottom: false,
        follow: None,
        content_height_bind: None,
        max_scroll_bind: None,
        measured: false,
        estimate: None,
        location: core::panic::Location::caller(),
        scrollbar: false,
        style_accumulator: crate::style::Style::new(),
    }
}

impl<T, R> VirtualListBuilder<T, R>
where
    T: 'static,
    R: RowRender<T>,
{
    /// Applies the identified option.
    #[must_use]
    pub fn identified(self) -> Self
    where
        T: Identifiable,
    {
        self.key_by(|_, item| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            item.id().hash(&mut hasher);
            hasher.finish()
        })
    }

    /// Sets the key by.
    #[must_use]
    pub fn key_by(mut self, f: impl Fn(usize, &T) -> u64 + 'static) -> Self {
        self.row_key = Some(std::rc::Rc::new(f));
        self
    }

    /// Sets the row height.
    #[must_use]
    pub fn row_height(mut self, length: impl crate::layout::IntoPx) -> Self {
        self.row_height = length.into_px();
        self
    }

    /// Applies the measured option.
    #[must_use]
    pub const fn measured(mut self) -> Self {
        self.measured = true;
        self
    }

    /// Sets the estimate.
    #[must_use]
    pub fn estimate(mut self, length: impl crate::layout::IntoPx) -> Self {
        self.estimate = Some(length.into_px());
        self
    }

    /// Sets the item height.
    #[must_use]
    pub fn item_height(mut self, f: impl Fn(usize, &T) -> f32 + 'static) -> Self {
        self.item_height = Some(Box::new(f));
        self.row_tops.clear();
        self
    }

    /// Sets the viewport height.
    #[must_use]
    pub fn viewport_height(mut self, length: impl crate::layout::IntoPx) -> Self {
        self.viewport_height = length.into_px();
        self
    }

    /// Sets the overscan.
    #[must_use]
    pub const fn overscan(mut self, pages: usize) -> Self {
        self.overscan = pages;
        self
    }

    /// Sets the viewport width.
    #[must_use]
    pub const fn viewport_width(mut self, px: f32) -> Self {
        self.viewport_width = Some(px);
        self
    }

    /// Applies the viewport fill option.
    #[must_use]
    pub const fn viewport_fill(mut self) -> Self {
        self.sizing = ListSizing::Fill;
        self
    }

    /// Applies the shrink to content option.
    #[must_use]
    pub const fn shrink_to_content(mut self) -> Self {
        self.sizing = ListSizing::ShrinkToContent;
        self
    }

    /// Sets the scroll bind.
    #[must_use]
    pub const fn scroll_bind(mut self, scroll_y: Signal<f32>) -> Self {
        self.scroll_y = Some(scroll_y);
        self
    }

    /// Sets the scroll x bind.
    #[must_use]
    pub const fn scroll_x_bind(
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

    /// Sets the cursor.
    #[must_use]
    pub const fn cursor(mut self, cursor: Signal<usize>) -> Self {
        self.focused_row = Some(cursor);
        if self.follow.is_none() {
            self.follow = Some(cursor);
        }
        self
    }

    /// Sets the selection.
    #[must_use]
    pub const fn selection(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    /// Returns the signal that holds the selection.
    #[must_use]
    pub fn selection_state(&mut self) -> Signal<SelectionState> {
        self.ensure_selection_state()
    }

    /// Sets the keyboard.
    #[must_use]
    pub const fn keyboard(mut self, enabled: bool) -> Self {
        self.keyboard_enabled = enabled;
        self
    }

    /// Sets the change.
    #[must_use]
    pub fn change(mut self, f: impl Fn(usize) + 'static) -> Self {
        let selection = self.ensure_selection_state();
        effect(move || {
            if let Some(index) = selection.get().selected_indices().next() {
                f(index);
            }
        });
        self
    }

    /// Sets the follow end.
    #[must_use]
    pub const fn follow_end(mut self, on: bool) -> Self {
        if on {
            self.initial_scroll = Some(ScrollAnchor::Bottom);
        }
        self.pin_bottom = on;
        self
    }

    /// Sets the scroll into view.
    #[must_use]
    pub const fn scroll_into_view(mut self, index: usize) -> Self {
        self.initial_scroll = Some(ScrollAnchor::Index(index));
        self
    }

    /// Sets the initial scroll.
    #[must_use]
    pub const fn initial_scroll(mut self, anchor: ScrollAnchor) -> Self {
        self.initial_scroll = match anchor {
            ScrollAnchor::Top => None,
            anchor => Some(anchor),
        };
        self
    }

    /// Sets the follow row.
    #[must_use]
    pub const fn follow_row(mut self, target: Signal<usize>) -> Self {
        self.follow = Some(target);
        self
    }
}

builder_impls!([T: 'static, R: RowRender<T>] VirtualListBuilder<T, R>: paint(style_accumulator), widget);

#[cfg(test)]
mod tests;
