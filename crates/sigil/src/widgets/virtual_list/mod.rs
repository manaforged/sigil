//! Virtual list widget with fixed-height row virtualization.
//!
//! The widget owns a scroll signal unless one is supplied by the host. On
//! build it reads the current scroll offset, derives the visible row range,
//! and only materializes rows in that range.

#![allow(clippy::missing_const_for_fn, clippy::mod_module_files)]

mod keyboard;
mod render;
mod selection;
mod viewport;
mod window;

use sigil_events::prelude::NodeEventsExt;
use sigil_kernel::{Cx, IntoNode, NodeBuilder, NodeId, container};
use sigil_layout::LayoutStyle;
use sigil_layout::prelude::LayoutBuilderExt;
use sigil_reactive::{Signal, derived, effect, signal};

use std::ops::Deref;

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::scroll::wheel;
use keyboard::{KeyboardRuntime, KeyboardRuntimeInit, attach_key_handlers};
use render::{
    HeightSpec, VisibleRenderSpec, build_visible_window, content_container, viewport_container,
    width_apply, width_spec,
};
use selection::SelectionRuntime;
pub use selection::{SelectionMode, SelectionState};
use viewport::{ViewportSize, apply_viewport_overrides};
use window::{
    VisibleWindow, prefix_row_top, row_top, total_height, total_height_variable, visible_window,
    visible_window_variable,
};

/// Default uniform row height in logical pixels.
#[allow(unreachable_pub)]
pub const DEFAULT_ROW_HEIGHT: f32 = 32.0;
/// Default viewport height in logical pixels.
#[allow(unreachable_pub)]
pub const DEFAULT_VIEWPORT_HEIGHT: f32 = 480.0;

type ItemHeightFn<T> = dyn Fn(usize, &T) -> f32 + 'static;

/// Runtime context passed to the `items_bind` thunk by
/// [`VirtualListBuilder::into_node_builder`] so the thunk — which was
/// built inside [`VirtualListBuilder::items_bind`] with the item-slice
/// bound already resolved — can build the reactive visible-window subtree
/// without needing access to `&mut self`.
struct BoundCtx<R> {
    render: R,
    row_height: f32,
    width: render::WidthSpec,
    selection: SelectionRuntime,
    scroll_y: Signal<f32>,
    viewport_height: f32,
}

/// Type-erased "build reactive visible window" thunk produced by
/// [`VirtualListBuilder::items_bind`]. The thunk encapsulates the
/// `S: AsRef<[T]>` bound so the rest of the builder body can stay generic
/// over `T: 'static` without forcing `Clone` on the static-`Vec` path.
type BoundVisibleThunk<R> = dyn FnOnce(BoundCtx<R>) -> NodeBuilder + 'static;

/// Bookkeeping captured at [`VirtualListBuilder::items_bind`] time:
/// the thunk that builds the reactive visible-window subtree plus the
/// initial item count (peeked from the signal without subscribing) so
/// [`VirtualListBuilder::into_node_builder`] can seed scroll, content
/// height, and keyboard `total_rows` from the same snapshot.
struct BoundItems<R> {
    thunk: Box<BoundVisibleThunk<R>>,
    initial_rows: usize,
    /// Live item count tracked from the bound signal — feeds the
    /// keyboard runtime so navigation clamps follow filter/sort
    /// mutations instead of the build-time snapshot.
    live_rows: Signal<usize>,
}

/// Pre-resolved per-axis runtime values shared between
/// `build_static` and `build_bound` so the dispatcher
/// `into_node_builder` stays under the 50-line hygiene cap.
#[derive(Clone, Copy)]
struct BuildAxes {
    row_height: f32,
    viewport_height: f32,
    width: render::WidthSpec,
    scroll_y: Signal<f32>,
    focused_row: Signal<usize>,
    selection_state: Signal<SelectionState>,
}

/// Borrow the bound signal's value as an item slice. The `Deref`
/// indirection lets `Rc<Vec<T>>` (deref → `Vec<T>` → `&[T]`) and
/// plain `Vec<T>` (deref → `[T]`) share one bound.
fn bound_slice<S, T>(value: &S) -> &[T]
where
    S: Deref,
    S::Target: AsRef<[T]>,
{
    (*value).as_ref()
}

/// Attach the wheel-scroll handler to the viewport root. `max_scroll`
/// is consulted per event so the bound (`items_bind`) path tracks the
/// live item count — a filter can shrink the content while the pointer
/// is hovering over the list.
fn attach_wheel(
    root: NodeBuilder,
    axes: BuildAxes,
    max_scroll: impl Fn() -> f32 + 'static,
) -> NodeBuilder {
    let scroll_y = axes.scroll_y;
    let viewport_height = axes.viewport_height;
    root.on_scroll(move |ev| {
        wheel(ev, scroll_y, viewport_height, max_scroll());
    })
}

/// Builder returned by [`virtual_list`].
pub struct VirtualListBuilder<T, R>
where
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
{
    items: Vec<T>,
    bound: Option<BoundItems<R>>,
    render: R,
    row_height: f32,
    viewport_height: f32,
    viewport_width: Option<f32>,
    viewport_fill: bool,
    scroll_y: Option<Signal<f32>>,
    focused_row: Option<Signal<usize>>,
    selection_state: Option<Signal<SelectionState>>,
    selection_mode: SelectionMode,
    keyboard_enabled: bool,
    item_height: Option<Box<ItemHeightFn<T>>>,
    row_tops: Vec<f32>,
    layout_overrides: LayoutOverrides,
    initial_scroll: Option<InitialScroll>,
}

/// Pre-build instruction for seeding the scroll signal.
#[derive(Clone, Copy)]
enum InitialScroll {
    End,
    Row(usize),
}

/// Anchor for [`VirtualListBuilder::initial_scroll`].
///
/// Selects where the viewport should be parked on first build:
/// * [`Top`](ScrollAnchor::Top) — start at offset 0.
/// * [`Bottom`](ScrollAnchor::Bottom) — start at the maximum scroll
///   offset (equivalent to the long-standing `scroll_to_end` helper).
/// * [`Index`](ScrollAnchor::Index) — start so the row at the supplied
///   index is at the top of the viewport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollAnchor {
    /// Park the viewport at offset 0.
    Top,
    /// Park the viewport at the last page (`content_height - viewport`).
    Bottom,
    /// Park the viewport at the supplied row index.
    Index(usize),
}

/// Build a virtual list.
///
/// Each item produces one row via `render`. Only rows whose vertical
/// range overlaps the viewport are materialized as kernel nodes.
#[must_use]
pub fn virtual_list<T, R>(items: Vec<T>, render: R) -> VirtualListBuilder<T, R>
where
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
{
    VirtualListBuilder {
        items,
        bound: None,
        render,
        row_height: DEFAULT_ROW_HEIGHT,
        viewport_height: DEFAULT_VIEWPORT_HEIGHT,
        viewport_width: None,
        viewport_fill: false,
        scroll_y: None,
        focused_row: None,
        selection_state: None,
        selection_mode: SelectionMode::None,
        keyboard_enabled: true,
        item_height: None,
        row_tops: Vec::new(),
        layout_overrides: LayoutOverrides::new(),
        initial_scroll: None,
    }
}

impl<T, R> VirtualListBuilder<T, R>
where
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
{
    /// Set the uniform row height in logical pixels (non-positive falls back to default).
    #[must_use]
    pub const fn row_height(mut self, px: f32) -> Self {
        self.row_height = px;
        self
    }

    /// Set a per-item row height function.
    ///
    /// When present, this overrides the uniform row height and builds a
    /// prefix-sum row-top index for O(log n) scroll offset lookup.
    #[must_use]
    pub fn item_height(mut self, f: impl Fn(usize, &T) -> f32 + 'static) -> Self {
        self.item_height = Some(Box::new(f));
        self.row_tops.clear();
        self
    }

    /// Set the viewport height in logical pixels (non-positive falls back to default).
    #[must_use]
    pub const fn viewport_height(mut self, px: f32) -> Self {
        self.viewport_height = px;
        self
    }

    /// Set the viewport width in logical pixels. When omitted the viewport
    /// stretches to `100%` of the host's layout constraint.
    #[must_use]
    pub const fn viewport_width(mut self, px: f32) -> Self {
        self.viewport_width = Some(px);
        self
    }

    /// Make the viewport inherit its parent's resolved content box on
    /// both axes (sets [`sigil_layout::Dimension::Fill`] for width and
    /// height). Overrides any explicit
    /// [`viewport_width`](Self::viewport_width) /
    /// [`viewport_height`](Self::viewport_height) settings; the layout
    /// engine then reads the parent pane's measured size at layout time.
    ///
    /// ```ignore
    /// use sigil::widgets::virtual_list;
    /// use sigil_kernel::container;
    /// let _list = virtual_list((0..1_000).collect(), |_, _| container())
    ///     .row_height(24.0)
    ///     .viewport_fill();
    /// ```
    #[must_use]
    pub const fn viewport_fill(mut self) -> Self {
        self.viewport_fill = true;
        self
    }

    /// Bind the list to an external scroll offset signal (default: internal `0.0`).
    #[must_use]
    pub const fn scroll_bind(mut self, scroll_y: Signal<f32>) -> Self {
        self.scroll_y = Some(scroll_y);
        self
    }

    /// Bind the list to an external focused row signal.
    ///
    /// Without this binding, the builder creates an internal `Signal<usize>`
    /// at build time and initializes it to `0`.
    #[must_use]
    pub const fn focused_row_bind(mut self, focused_row: Signal<usize>) -> Self {
        self.focused_row = Some(focused_row);
        self
    }

    /// Set the list selection mode.
    #[must_use]
    pub const fn selection(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    /// Return the live selection state signal.
    ///
    /// Calling this before build installs the same signal that row click
    /// handlers will update.
    #[must_use]
    pub fn selection_state(&mut self) -> Signal<SelectionState> {
        self.ensure_selection_state()
    }

    /// Enable keyboard navigation.
    ///
    /// Enabled by default. When disabled, the list root does not install
    /// intra-list key handlers.
    #[must_use]
    pub const fn keyboard(mut self, enabled: bool) -> Self {
        self.keyboard_enabled = enabled;
        self
    }

    /// Register a callback that fires whenever the first selected row
    /// changes. Must be called inside a reactive
    /// [`Runtime`](sigil_reactive::Runtime) scope.
    ///
    /// The underlying [`effect`](sigil_reactive::effect) runs the
    /// closure once at registration time to record subscriptions, so
    /// `f` fires ONCE synchronously during build if a selection is
    /// already present (pre-seeded selection-state signal or a prior
    /// [`selection_state`](Self::selection_state) call). Callers that
    /// only want *future* changes should branch on a first-call flag.
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

    /// Seed the scroll signal at `(content_height - viewport_height).max(0.0)`
    /// before first paint. Useful for "tail -f" log-style views.
    #[must_use]
    pub const fn scroll_to_end(mut self) -> Self {
        self.initial_scroll = Some(InitialScroll::End);
        self
    }

    /// Forward-compat placeholder — no-op while `items` is `Vec<T>`.
    /// Will wire a reactive reseat-on-grow effect once items become
    /// `Signal<Vec<T>>`.
    #[must_use]
    pub const fn auto_pin_to_bottom(self, _on: bool) -> Self {
        self
    }

    /// Bind the list's items to a signal whose value derefs to an item
    /// slice (`Signal<Vec<T>>`, `Signal<Rc<Vec<T>>>`, …).
    ///
    /// The visible window is wired through [`NodeBuilder::children_reactive`]
    /// so BOTH item-signal and `scroll_y` mutations re-render just the
    /// rows overlapping the viewport — in place, no whole-widget rebuild
    /// (5/5 Wave 2 T6-prereq; the previous full-vec render is gone).
    /// Selection state continues to live in the external
    /// [`Signal<SelectionState>`](SelectionState) — row identity is
    /// positional, matching `docs/specs/2026-05-20-children-reactive-design.md`
    /// §3.
    ///
    /// Calling `items_bind` overrides the `Vec<T>` passed to
    /// [`virtual_list`]; the static items list is ignored on the
    /// reactive path. Pass `Signal<Rc<Vec<T>>>` for a clone-free bind —
    /// `Rc`'s `Clone` is O(1), so 100k-row tables never copy row storage
    /// on the reactive path.
    ///
    /// The keyboard runtime reads the LIVE item count derived from this
    /// signal, so Home/End/Page clamps follow signal mutations (e.g. a
    /// table quick-filter narrowing the visible rows).
    #[must_use]
    pub fn items_bind<S>(mut self, signal: Signal<S>) -> Self
    where
        S: Deref + Clone + 'static,
        S::Target: AsRef<[T]>,
    {
        let initial_rows = bound_slice(&signal.peek()).len();
        let live_rows = derived(move || bound_slice(&signal.get()).len());
        // Captures `S: Clone` so the outer builder body can stay
        // `T: 'static`. The thunk receives `BoundCtx<R>` at
        // into_node_builder time and produces the reactive
        // visible-window NodeBuilder.
        let thunk: Box<BoundVisibleThunk<R>> = Box::new(move |ctx: BoundCtx<R>| {
            let BoundCtx {
                render,
                row_height,
                width,
                selection,
                scroll_y,
                viewport_height,
            } = ctx;
            container()
                .layout(width_apply(width, LayoutStyle::block()))
                .children_reactive(move || {
                    let items = signal.get();
                    let slice = bound_slice(&items);
                    let total = slice.len();
                    let window =
                        window::visible_window(scroll_y.get(), total, row_height, viewport_height);
                    let visible = build_visible_window(
                        slice,
                        &render,
                        window,
                        VisibleRenderSpec::new((row_height, None), width, selection),
                    );
                    vec![
                        content_container(window::total_height(total, row_height), width)
                            .child(visible),
                    ]
                })
        });
        self.bound = Some(BoundItems {
            thunk,
            initial_rows,
            live_rows,
        });
        self
    }

    /// Pre-seed the scroll position so row `index` is visible at build time.
    ///
    /// Overrides any prior [`scroll_to_end`](Self::scroll_to_end) call.
    #[must_use]
    pub const fn scroll_into_view(mut self, index: usize) -> Self {
        self.initial_scroll = Some(InitialScroll::Row(index));
        self
    }

    /// Seat the scroll position on first build using the supplied anchor.
    ///
    /// Unifies [`scroll_to_end`](Self::scroll_to_end) and
    /// [`scroll_into_view`](Self::scroll_into_view) under one entrypoint
    /// keyed by [`ScrollAnchor`]:
    ///
    /// * [`ScrollAnchor::Top`] — park at offset 0.
    /// * [`ScrollAnchor::Bottom`] — park at the last page.
    /// * [`ScrollAnchor::Index(n)`](ScrollAnchor::Index) — park so row
    ///   `n` is at the top of the viewport.
    ///
    /// Overrides any prior anchor / `scroll_to_end` / `scroll_into_view`
    /// on the same builder.
    ///
    /// ```
    /// use sigil::prelude::*;
    /// use sigil::widgets::{virtual_list, ScrollAnchor};
    /// let items = (0..200u32).collect::<Vec<_>>();
    /// let _b = virtual_list(items, |_, n| container().child(text(n.to_string())))
    ///     .initial_scroll(ScrollAnchor::Bottom);
    /// ```
    #[must_use]
    pub const fn initial_scroll(mut self, anchor: ScrollAnchor) -> Self {
        self.initial_scroll = match anchor {
            ScrollAnchor::Top => None,
            ScrollAnchor::Bottom => Some(InitialScroll::End),
            ScrollAnchor::Index(n) => Some(InitialScroll::Row(n)),
        };
        self
    }

    /// Re-seat the scroll position to keep `target` visible whenever the
    /// supplied signal mutates.
    ///
    /// Installs a [`sigil_reactive::effect`] that reads `target.get()`
    /// and re-pins the scroll offset to that row top on every change,
    /// clamped to the maximum scroll. The effect is registered at build
    /// time and lives for the surrounding [`Runtime`](sigil_reactive::Runtime)
    /// scope.
    ///
    /// Use for "follow the cursor" / "scroll to selected" UX where the
    /// host owns a `Signal<usize>` cursor and wants the list to chase it.
    /// Combine with [`initial_scroll`](Self::initial_scroll) to seed the
    /// starting position.
    ///
    /// ```no_run
    /// use sigil::prelude::*;
    /// use sigil::widgets::virtual_list;
    /// let cursor = signal(42usize);
    /// let items = (0..200u32).collect::<Vec<_>>();
    /// let _b = virtual_list(items, |_, n| container().child(text(n.to_string())))
    ///     .follow(cursor);
    /// ```
    #[must_use]
    pub fn follow(mut self, target: Signal<usize>) -> Self {
        let scroll_y = self.scroll_y.unwrap_or_else(|| signal(0.0));
        self.scroll_y = Some(scroll_y);
        let row_height = window::positive_px(self.row_height, DEFAULT_ROW_HEIGHT);
        let viewport_height = window::positive_px(self.viewport_height, DEFAULT_VIEWPORT_HEIGHT);
        let total_rows = self.items.len();
        // Variable-height follow is harder (need row_tops at effect time);
        // for the uniform-height path we can compute the row top directly.
        // Variable-height follow is a follow-up — flagged inline.
        effect(move || {
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
        let focused_row = self.focused_row.take().unwrap_or_else(|| signal(0));
        let selection_state = self.ensure_selection_state();

        if self.bound.is_some() {
            return self.build_bound(BuildAxes {
                row_height,
                viewport_height,
                width,
                scroll_y,
                focused_row,
                selection_state,
            });
        }
        self.build_static(BuildAxes {
            row_height,
            viewport_height,
            width,
            scroll_y,
            focused_row,
            selection_state,
        })
    }

    fn build_bound(mut self, axes: BuildAxes) -> NodeBuilder {
        let bound = self
            .bound
            .take()
            .expect("build_bound called without items_bind");
        let selection = SelectionRuntime::new(axes.selection_state);
        // The thunk's `children_reactive` rebuilds the content subtree
        // (sized to the CURRENT total each time) with only the visible
        // window materialized — items and scroll subscriptions both live
        // inside it, so there is no build-time height snapshot to go stale.
        let content = (bound.thunk)(BoundCtx {
            render: self.render,
            row_height: axes.row_height,
            width: axes.width,
            selection,
            scroll_y: axes.scroll_y,
            viewport_height: axes.viewport_height,
        });
        let initial_rows = bound.initial_rows;
        let live_rows = bound.live_rows;
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
        if self.keyboard_enabled {
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
        } else {
            root
        }
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
            visible_window_variable(scroll_y, &self.row_tops, vh)
        } else {
            visible_window(scroll_y, self.items.len(), row_height, vh)
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

    /// Build row-top index if variable heights are active, apply any pending
    /// initial-scroll request to `scroll_y`, and return the variable flag.
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
