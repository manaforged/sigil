use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

use sigil_events::prelude::NodeEventsExt;
use sigil_kernel::{NodeBuilder, NodeId, Role, container, schedule_keyed_rebuild};
use sigil_layout::LayoutReactiveExt;
use sigil_layout::prelude::{Dimension, LayoutBuilderExt, LayoutStyle, Overflow, Position};
use sigil_reactive::{Signal, derived, effect_named};

use super::selection::SelectionRuntime;
use super::window::{
    VisibleWindow, prefix_row_top, row_top, strip_height, total_height, visible_window,
};

pub(super) type HeightSpec<'a> = (f32, Option<&'a [f32]>);

/// Live keyed rows shared with the strip effect: key → mounted node.
type MountedRows = RefCell<Vec<(u64, NodeId)>>;

#[derive(Clone, Copy)]
pub(super) struct VisibleRenderSpec<'a> {
    heights: HeightSpec<'a>,
    width: WidthSpec,
    selection: SelectionRuntime,
    scroll_y: f32,
}

#[derive(Clone, Copy)]
pub(super) enum WidthSpec {
    Fixed(f32),
    Stretch,
}

pub(super) fn viewport_container(height: f32, width: WidthSpec) -> NodeBuilder {
    container().role(Role::ScrollView).layout(
        width.apply(
            LayoutStyle::block()
                .height(height)
                .overflow_y(Overflow::Hidden),
        ),
    )
}

pub(super) fn content_container(height: f32, width: WidthSpec) -> NodeBuilder {
    container().layout(width.apply(LayoutStyle::block().height(height)))
}

pub(super) fn build_visible_window<T, R>(
    items: &[T],
    render: &R,
    window: VisibleWindow,
    spec: VisibleRenderSpec<'_>,
) -> NodeBuilder
where
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
{
    let (row_height, row_tops) = spec.heights;
    let document_top = row_tops.map_or_else(
        || row_top(window.first, row_height),
        |row_tops| prefix_row_top(row_tops, window.first),
    );
    // Park the strip in viewport space so overflow clip still intersects.
    let top = document_top - spec.scroll_y;
    let visible_height = row_tops.map_or_else(
        || total_height(window.len(), row_height),
        |row_tops| prefix_row_top(row_tops, window.end) - document_top,
    );
    let mut visible = container().layout(
        spec.width.apply(
            LayoutStyle::flex_column()
                .position(Position::Absolute)
                .inset_top(top)
                .inset_left(0.0)
                .height(visible_height),
        ),
    );

    for index in window.first..window.end {
        let Some(item) = items.get(index) else {
            break;
        };
        let slot_height = row_tops.map_or(row_height, |row_tops| {
            prefix_row_top(row_tops, index + 1) - prefix_row_top(row_tops, index)
        });
        visible = visible.child(place_row(
            index,
            slot_height,
            spec.width,
            render(index, item),
            spec.selection,
        ));
    }

    visible
}

pub(super) fn place_row(
    index: usize,
    row_height: f32,
    width: WidthSpec,
    row: NodeBuilder,
    selection: SelectionRuntime,
) -> NodeBuilder {
    if selection.idle() {
        return row;
    }
    row_slot(index, row_height, width, row, selection)
}

fn row_slot(
    index: usize,
    row_height: f32,
    width: WidthSpec,
    row: NodeBuilder,
    selection: SelectionRuntime,
) -> NodeBuilder {
    let mut slot = container()
        .layout(width.apply(LayoutStyle::block().height(row_height)))
        .child(row);
    if let Some(selected) = selection.selected_signal(index) {
        slot = slot.selected(selected);
    }
    slot.pointer_down(move |event| {
        selection.apply_pointer_down(index, event);
    })
}

impl WidthSpec {
    fn apply(self, style: LayoutStyle) -> LayoutStyle {
        match self {
            Self::Fixed(px) => style.width(px),
            Self::Stretch => style.width(Dimension::Percent(1.0)),
        }
    }
}

/// Module-public adapter for [`WidthSpec::apply`] so callers in
/// `mod.rs` can decorate a [`LayoutStyle`] without exposing the
/// per-axis enum's private method.
pub(super) fn width_apply(width: WidthSpec, style: LayoutStyle) -> LayoutStyle {
    width.apply(style)
}

pub(super) fn width_spec(width: Option<f32>) -> WidthSpec {
    match width {
        Some(px) if px.is_finite() && px > 0.0 => WidthSpec::Fixed(px),
        _ => WidthSpec::Stretch,
    }
}

impl<'a> VisibleRenderSpec<'a> {
    pub(super) const fn new(
        heights: HeightSpec<'a>,
        width: WidthSpec,
        selection: SelectionRuntime,
        scroll_y: f32,
    ) -> Self {
        Self {
            heights,
            width,
            selection,
            scroll_y,
        }
    }
}

/// Spec for the bound strip: keyed visible rows + viewport offset.
pub(super) struct BoundStrip<T, R> {
    pub render: Rc<R>,
    pub row_height: f32,
    pub viewport_height: f32,
    pub width: WidthSpec,
    pub selection: SelectionRuntime,
    pub live_len: Signal<usize>,
    pub row_key: Option<super::RowKey<T>>,
}

/// Stable content child: keyed rows, strip parked at `first*rh - scroll`.
pub(super) fn bound_strip<S, T, R>(
    signal: Signal<S>,
    scroll_y: Signal<f32>,
    spec: BoundStrip<T, R>,
) -> NodeBuilder
where
    S: Deref + Clone + 'static,
    S::Target: AsRef<[T]>,
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
{
    let rh = spec.row_height;
    let vh = spec.viewport_height;
    let pos = derived(move || strip_offset(scroll_y.get(), rh, vh));
    let height = strip_height(vh, rh);
    let first = keyed_rows(signal, scroll_y.get(), &spec, 0, None);
    let seed: Vec<u64> = first.iter().map(|(key, _)| *key).collect();
    let mut strip = container()
        .layout(
            spec.width.apply(
                LayoutStyle::flex_column()
                    .position(Position::Absolute)
                    .inset_left(0.0)
                    .height(height),
            ),
        )
        .position_reactive(pos);
    for (_, row) in first {
        strip = strip.child(row);
    }
    strip.on_added(move |cx, parent| {
        let kids = cx.get(parent).map(|node| node.children().to_vec());
        let kids = kids.unwrap_or_default();
        let prior = if seed.len() == kids.len() {
            Rc::new(RefCell::new(seed.into_iter().zip(kids).collect()))
        } else {
            Rc::new(RefCell::new(Vec::new()))
        };
        let primed = Cell::new(true);
        // Seeded with the build-time length so the priming run does not
        // bump the generation out from under the seeded keys.
        let last_len = Cell::new(spec.live_len.peek());
        let generation = Cell::new(0_u64);
        effect_named("sigil-virtual-list::strip", move || {
            let len = spec.live_len.get();
            let y = scroll_y.get();
            if spec.row_key.is_some() {
                let _ = signal.get();
            } else if last_len.replace(len) != len {
                // Index keys name a list SLOT: when the count changes the
                // slots hold new items, so salt the keys to force a remount.
                // Reusing the mounted node would paint the previous
                // occupant's content (stale rows after a filter shrink).
                generation.set(generation.get() + 1);
            }
            if primed.replace(false) {
                return;
            }
            let incoming = keyed_rows(signal, y, &spec, generation.get(), Some(&prior));
            schedule_keyed_rebuild(parent, Rc::clone(&prior), incoming);
        });
    })
}

fn strip_offset(scroll_y: f32, row_height: f32, viewport_height: f32) -> (f32, f32) {
    if row_height <= 0.0 {
        return (0.0, 0.0);
    }
    let first = visible_window(scroll_y, usize::MAX, row_height, viewport_height).first;
    #[allow(clippy::cast_precision_loss)]
    let first_top = first as f32 * row_height;
    (0.0, first_top - scroll_y)
}

fn keyed_rows<S, T, R>(
    signal: Signal<S>,
    scroll_y: f32,
    spec: &BoundStrip<T, R>,
    generation: u64,
    mounted: Option<&MountedRows>,
) -> Vec<(u64, NodeBuilder)>
where
    S: Deref + Clone + 'static,
    S::Target: AsRef<[T]>,
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
{
    let items = signal.peek();
    let slice: &[T] = (*items).as_ref();
    let window = visible_window(scroll_y, slice.len(), spec.row_height, spec.viewport_height);
    let mut rows = Vec::with_capacity(window.len());
    for index in window.first..window.end {
        let Some(item) = slice.get(index) else {
            break;
        };
        let key = spec.row_key.as_ref().map_or_else(
            // Slot keys live in a per-count generation so a length change
            // never matches a stale mounted row.
            || (generation << 32) | index as u64,
            |key_of| key_of(index, item),
        );
        // A cheap placeholder stands in for a key that is already MOUNTED:
        // `apply_keyed` reuses the live node and drops the builder. The
        // decision must come from the mounted map, not from the previous
        // window's keys: the strip effect can re-run (and re-enqueue a
        // replacement rebuild) before the pending one is applied, and a
        // placeholder for a key that was never mounted would mount as an
        // empty, zero-height row — the filtered-table blank-window bug.
        let kept = mounted.is_some_and(|mounted| mounted.borrow().iter().any(|(k, _)| *k == key));
        if kept {
            rows.push((key, container()));
            continue;
        }
        rows.push((
            key,
            place_row(
                index,
                spec.row_height,
                spec.width,
                (*spec.render)(index, item),
                spec.selection,
            ),
        ));
    }
    rows
}
