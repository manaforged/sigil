#![allow(clippy::missing_const_for_fn, clippy::mod_module_files)]

mod keyboard;
mod render;
mod scroll;
mod selection;
mod viewport;
mod window;

use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, container};
use crate::layout::LayoutStyle;
use crate::layout::prelude::LayoutBuilderExt;
use crate::reactive::{Signal, derived_eq, effect, signal};

use std::ops::Deref;

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::scroll::WheelX;
use super::table::Identifiable;
use keyboard::{KeyboardRuntime, KeyboardRuntimeInit, attach_key_handlers};
use render::{
    BoundStrip, HeightSpec, VisibleRenderSpec, bound_strip, build_visible_window,
    content_container, viewport_container, width_apply, width_spec,
};
use scroll::{apply_bound_scroll, attach_wheel};
use selection::SelectionRuntime;
pub use selection::{SelectionMode, SelectionState};
use viewport::{ViewportSize, apply_viewport_overrides};
use window::{
    VisibleWindow, prefix_row_top, row_top, strip_height, total_height, total_height_variable,
    visible_window, visible_window_variable,
};

#[allow(unreachable_pub, reason = "intentional, reviewed trade-off")]
pub const DEFAULT_ROW_HEIGHT: f32 = 32.0;
#[allow(unreachable_pub, reason = "intentional, reviewed trade-off")]
pub const DEFAULT_VIEWPORT_HEIGHT: f32 = 480.0;
const DEFAULT_OVERSCAN: usize = 1;

type ItemHeightFn<T> = dyn Fn(usize, &T) -> f32 + 'static;
pub(super) type RowKey<T> = std::rc::Rc<dyn Fn(usize, &T) -> u64>;

struct BoundCtx<T, R> {
    render: R,
    row_height: f32,
    width: render::WidthSpec,
    selection: SelectionRuntime,
    scroll_y: Signal<f32>,
    viewport_height: f32,
    overscan: usize,
    row_key: Option<RowKey<T>>,
}

type BoundVisibleThunk<T, R> = dyn FnOnce(BoundCtx<T, R>) -> NodeBuilder + 'static;

struct BoundItems<T, R> {
    thunk: Box<BoundVisibleThunk<T, R>>,
    initial_rows: usize,
    live_rows: Signal<usize>,
}

#[derive(Clone, Copy)]
struct BuildAxes {
    row_height: f32,
    viewport_height: f32,
    width: render::WidthSpec,
    scroll_y: Signal<f32>,
    scroll_x: Option<WheelX>,
    focused_row: Signal<usize>,
    selection_state: Signal<SelectionState>,
}

fn bound_slice<S, T>(value: &S) -> &[T]
where
    S: Deref,
    S::Target: AsRef<[T]>,
{
    (*value).as_ref()
}

pub struct VirtualListBuilder<T, R>
where
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
{
    items: Vec<T>,
    bound: Option<BoundItems<T, R>>,
    render: R,
    row_key: Option<RowKey<T>>,
    row_height: f32,
    viewport_height: f32,
    viewport_width: Option<f32>,
    viewport_fill: bool,
    overscan: usize,
    scroll_y: Option<Signal<f32>>,
    scroll_x: Option<WheelX>,
    focused_row: Option<Signal<usize>>,
    selection_state: Option<Signal<SelectionState>>,
    selection_mode: SelectionMode,
    keyboard_enabled: bool,
    item_height: Option<Box<ItemHeightFn<T>>>,
    row_tops: Vec<f32>,
    layout_overrides: LayoutOverrides,
    initial_scroll: Option<InitialScroll>,
    pin_bottom: bool,
}

#[derive(Clone, Copy)]
enum InitialScroll {
    End,
    Row(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollAnchor {
    Top,
    Bottom,
    Index(usize),
}

pub trait IntoList<T, R: Fn(usize, &T) -> NodeBuilder + 'static> {
    fn into_list(self, render: R) -> VirtualListBuilder<T, R>;
}

impl<T, R> IntoList<T, R> for Vec<T>
where
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
{
    fn into_list(self, render: R) -> VirtualListBuilder<T, R> {
        from_vec(self, render)
    }
}

impl<S, T, R> IntoList<T, R> for Signal<S>
where
    S: Deref + Clone + 'static,
    S::Target: AsRef<[T]>,
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
{
    fn into_list(self, render: R) -> VirtualListBuilder<T, R> {
        from_vec(Vec::new(), render).bind_items(self)
    }
}

#[must_use]
pub fn virtual_list<T, R, I>(items: I, render: R) -> VirtualListBuilder<T, R>
where
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
    I: IntoList<T, R>,
{
    items.into_list(render)
}

fn from_vec<T, R>(items: Vec<T>, render: R) -> VirtualListBuilder<T, R>
where
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
{
    VirtualListBuilder {
        items,
        bound: None,
        render,
        row_key: None,
        row_height: DEFAULT_ROW_HEIGHT,
        viewport_height: DEFAULT_VIEWPORT_HEIGHT,
        viewport_width: None,
        viewport_fill: false,
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
    }
}

#[must_use]
pub fn virtual_list_bind<S, T, R>(items: Signal<S>, render: R) -> VirtualListBuilder<T, R>
where
    S: Deref + Clone + 'static,
    S::Target: AsRef<[T]>,
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
{
    virtual_list(items, render)
}

impl<T, R> VirtualListBuilder<T, R>
where
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
{
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

    #[must_use]
    pub fn key_by(mut self, f: impl Fn(usize, &T) -> u64 + 'static) -> Self {
        self.row_key = Some(std::rc::Rc::new(f));
        self
    }

    #[must_use]
    pub const fn row_height(mut self, px: f32) -> Self {
        self.row_height = px;
        self
    }

    #[must_use]
    pub fn item_height(mut self, f: impl Fn(usize, &T) -> f32 + 'static) -> Self {
        self.item_height = Some(Box::new(f));
        self.row_tops.clear();
        self
    }

    #[must_use]
    pub const fn viewport_height(mut self, px: f32) -> Self {
        self.viewport_height = px;
        self
    }

    #[must_use]
    pub const fn overscan(mut self, pages: usize) -> Self {
        self.overscan = pages;
        self
    }

    #[must_use]
    pub const fn viewport_width(mut self, px: f32) -> Self {
        self.viewport_width = Some(px);
        self
    }

    #[must_use]
    pub const fn viewport_fill(mut self) -> Self {
        self.viewport_fill = true;
        self
    }

    #[must_use]
    pub const fn scroll_bind(mut self, scroll_y: Signal<f32>) -> Self {
        self.scroll_y = Some(scroll_y);
        self
    }

    #[must_use]
    pub const fn scroll_x_bind(
        mut self,
        scroll_x: Signal<f32>,
        max: Signal<f32>,
        width: Signal<f32>,
    ) -> Self {
        self.scroll_x = Some((scroll_x, max, width));
        self
    }

    #[must_use]
    pub const fn focused_row_bind(mut self, focused_row: Signal<usize>) -> Self {
        self.focused_row = Some(focused_row);
        self
    }

    #[must_use]
    pub const fn selection(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    #[must_use]
    pub fn selection_state(&mut self) -> Signal<SelectionState> {
        self.ensure_selection_state()
    }

    #[must_use]
    pub const fn keyboard(mut self, enabled: bool) -> Self {
        self.keyboard_enabled = enabled;
        self
    }

    #[must_use]
    pub fn select(mut self, f: impl Fn(usize) + 'static) -> Self {
        let selection = self.ensure_selection_state();
        effect(move || {
            if let Some(index) = selection.get().selected_indices().next() {
                f(index);
            }
        });
        self
    }

    #[must_use]
    pub const fn scroll_to_end(mut self) -> Self {
        self.initial_scroll = Some(InitialScroll::End);
        self
    }

    #[must_use]
    pub const fn auto_pin_to_bottom(mut self, on: bool) -> Self {
        self.pin_bottom = on;
        self
    }

    fn bind_items<S>(mut self, signal: Signal<S>) -> Self
    where
        S: Deref + Clone + 'static,
        S::Target: AsRef<[T]>,
    {
        let initial_rows = bound_slice(&signal.peek()).len();
        let live_rows = derived_eq(move || bound_slice(&signal.get()).len());
        let thunk: Box<BoundVisibleThunk<T, R>> = Box::new(move |ctx: BoundCtx<T, R>| {
            let BoundCtx {
                render,
                row_height,
                width,
                selection,
                scroll_y,
                viewport_height,
                overscan,
                row_key,
            } = ctx;
            let render = std::rc::Rc::new(render);
            let live_len = live_rows;
            container()
                .layout(width_apply(width, LayoutStyle::block()))
                .child(
                    content_container(strip_height(viewport_height, row_height, overscan), width)
                        .child(bound_strip(
                            signal,
                            scroll_y,
                            BoundStrip {
                                render: std::rc::Rc::clone(&render),
                                row_height,
                                viewport_height,
                                overscan,
                                width,
                                selection,
                                live_len,
                                row_key: row_key.clone(),
                            },
                        )),
                )
        });
        self.bound = Some(BoundItems {
            thunk,
            initial_rows,
            live_rows,
        });
        self
    }

    #[must_use]
    pub const fn scroll_into_view(mut self, index: usize) -> Self {
        self.initial_scroll = Some(InitialScroll::Row(index));
        self
    }

    #[must_use]
    pub const fn initial_scroll(mut self, anchor: ScrollAnchor) -> Self {
        self.initial_scroll = match anchor {
            ScrollAnchor::Top => None,
            ScrollAnchor::Bottom => Some(InitialScroll::End),
            ScrollAnchor::Index(n) => Some(InitialScroll::Row(n)),
        };
        self
    }

    #[must_use]
    pub fn follow(mut self, target: Signal<usize>) -> Self {
        let scroll_y = self.scroll_y.unwrap_or_else(|| signal(0.0));
        self.scroll_y = Some(scroll_y);
        let row_height = window::positive_px(self.row_height, DEFAULT_ROW_HEIGHT);
        let viewport_height = window::positive_px(self.viewport_height, DEFAULT_VIEWPORT_HEIGHT);
        let live_rows = self.bound.as_ref().map(|bound| bound.live_rows);
        let static_rows = self.items.len();
        effect(move || {
            let total_rows = live_rows.map_or(static_rows, |rows| rows.get());
            let idx = target.get().min(total_rows.saturating_sub(1));
            let target_top = row_top(idx, row_height);
            let max = (total_height(total_rows, row_height) - viewport_height).max(0.0);
            scroll_y.set(target_top.clamp(0.0, max));
        });
        self
    }

    fn into_node_builder(mut self) -> NodeBuilder {
        let row_height = window::positive_px(self.row_height, DEFAULT_ROW_HEIGHT);
        let viewport_height = window::positive_px(self.viewport_height, DEFAULT_VIEWPORT_HEIGHT);
        let width = width_spec(self.viewport_width);
        let scroll_y = self.scroll_y.take().unwrap_or_else(|| signal(0.0));
        let scroll_x = self.scroll_x;
        let focused_row = self.focused_row.take().unwrap_or_else(|| signal(0));
        let selection_state = self.ensure_selection_state();

        if self.bound.is_some() {
            return self.build_bound(BuildAxes {
                row_height,
                viewport_height,
                width,
                scroll_y,
                scroll_x,
                focused_row,
                selection_state,
            });
        }
        self.build_static(BuildAxes {
            row_height,
            viewport_height,
            width,
            scroll_y,
            scroll_x,
            focused_row,
            selection_state,
        })
    }

    fn build_bound(mut self, axes: BuildAxes) -> NodeBuilder {
        let bound = self
            .bound
            .take()
            .expect("build_bound called without virtual_list_bind");
        let selection = SelectionRuntime::new(axes.selection_state);
        let content = (bound.thunk)(BoundCtx {
            render: self.render,
            row_height: axes.row_height,
            width: axes.width,
            selection,
            scroll_y: axes.scroll_y,
            viewport_height: axes.viewport_height,
            overscan: self.overscan,
            row_key: self.row_key,
        });
        let initial_rows = bound.initial_rows;
        let live_rows = bound.live_rows;
        apply_bound_scroll(
            axes,
            self.initial_scroll,
            self.pin_bottom,
            initial_rows,
            live_rows,
        );
        let root = viewport_container(axes.viewport_height, axes.width).child(content);
        let root = apply_viewport_overrides(
            root,
            &self.layout_overrides,
            ViewportSize {
                height: axes.viewport_height,
                width: self.viewport_width,
                fill: self.viewport_fill,
            },
        );
        let root = attach_wheel(root, axes, move || {
            (total_height(live_rows.get(), axes.row_height) - axes.viewport_height).max(0.0)
        });
        attach_keyboard(root, axes, initial_rows, live_rows, self.keyboard_enabled)
    }

    fn build_static(mut self, axes: BuildAxes) -> NodeBuilder {
        let using_variable_heights =
            self.prepare_scroll(axes.scroll_y, axes.row_height, axes.viewport_height);
        let window = self.current_window(
            axes.scroll_y.get(),
            axes.row_height,
            axes.viewport_height,
            using_variable_heights,
        );
        let content_height = self.content_height(axes.row_height, using_variable_heights);
        let heights = self.height_spec(axes.row_height, using_variable_heights);
        let visible = build_visible_window(
            &self.items,
            &self.render,
            window,
            VisibleRenderSpec::new(
                heights,
                axes.width,
                SelectionRuntime::new(axes.selection_state),
                axes.scroll_y.get(),
            ),
        );
        let content = content_container(content_height, axes.width).child(visible);
        let root = viewport_container(axes.viewport_height, axes.width).child(content);
        let root = apply_viewport_overrides(
            root,
            &self.layout_overrides,
            ViewportSize {
                height: axes.viewport_height,
                width: self.viewport_width,
                fill: self.viewport_fill,
            },
        );
        let max_scroll = (content_height - axes.viewport_height).max(0.0);
        let root = attach_wheel(root, axes, move || max_scroll);
        if self.keyboard_enabled {
            attach_key_handlers(
                root,
                KeyboardRuntime::new(KeyboardRuntimeInit {
                    total_rows: self.items.len(),
                    total_rows_live: None,
                    row_height: axes.row_height,
                    viewport_height: axes.viewport_height,
                    scroll_y: axes.scroll_y,
                    focused_row: axes.focused_row,
                    row_tops: using_variable_heights.then(|| self.row_tops.clone()),
                }),
            )
        } else {
            root
        }
    }

    fn current_window(
        &self,
        scroll_y: f32,
        row_height: f32,
        vh: f32,
        variable: bool,
    ) -> VisibleWindow {
        if variable {
            visible_window_variable(scroll_y, &self.row_tops, vh, self.overscan)
        } else {
            visible_window(scroll_y, self.items.len(), row_height, vh, self.overscan)
        }
    }

    fn content_height(&self, row_height: f32, variable: bool) -> f32 {
        if variable {
            total_height_variable(&self.row_tops)
        } else {
            total_height(self.items.len(), row_height)
        }
    }

    fn height_spec(&self, row_height: f32, variable: bool) -> HeightSpec<'_> {
        (row_height, variable.then_some(self.row_tops.as_slice()))
    }

    fn prepare_scroll(&mut self, scroll_y: Signal<f32>, row_height: f32, vh: f32) -> bool {
        let variable = self.item_height.is_some();
        if variable {
            self.ensure_row_tops();
        }
        let Some(initial) = self.initial_scroll else {
            return variable;
        };
        let max = (self.content_height(row_height, variable) - vh).max(0.0);
        let target = match initial {
            InitialScroll::End => max,
            InitialScroll::Row(index) if variable => prefix_row_top(&self.row_tops, index),
            InitialScroll::Row(index) => row_top(index, row_height),
        };
        scroll_y.set(target.clamp(0.0, max));
        variable
    }

    fn ensure_row_tops(&mut self) {
        if self.row_tops.len() == self.items.len().saturating_add(1) {
            return;
        }
        self.row_tops.clear();
        self.row_tops.reserve(self.items.len().saturating_add(1));
        self.row_tops.push(0.0);
        let Some(item_height) = &self.item_height else {
            return;
        };
        let mut cursor = 0.0;
        for (index, item) in self.items.iter().enumerate() {
            cursor += window::positive_px(item_height(index, item), DEFAULT_ROW_HEIGHT);
            self.row_tops.push(cursor);
        }
    }

    fn ensure_selection_state(&mut self) -> Signal<SelectionState> {
        if let Some(state) = self.selection_state {
            state.update(|state| {
                if state.mode() != self.selection_mode {
                    *state = SelectionState::new(self.selection_mode);
                }
            });
            state
        } else {
            let state = signal(SelectionState::new(self.selection_mode));
            self.selection_state = Some(state);
            state
        }
    }
}

fn attach_keyboard(
    root: NodeBuilder,
    axes: BuildAxes,
    initial_rows: usize,
    live_rows: Signal<usize>,
    keyboard_enabled: bool,
) -> NodeBuilder {
    if !keyboard_enabled {
        return root;
    }
    attach_key_handlers(
        root,
        KeyboardRuntime::new(KeyboardRuntimeInit {
            total_rows: initial_rows,
            total_rows_live: Some(live_rows),
            row_height: axes.row_height,
            viewport_height: axes.viewport_height,
            scroll_y: axes.scroll_y,
            focused_row: axes.focused_row,
            row_tops: None,
        }),
    )
}

impl<T, R> IntoNode for VirtualListBuilder<T, R>
where
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
{
    fn build(self, cx: &mut Cx) -> NodeId {
        self.into_node_builder().build(cx)
    }
}

impl<T, R> WidgetLayoutExt for VirtualListBuilder<T, R>
where
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
{
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

#[cfg(test)]
mod tests;
