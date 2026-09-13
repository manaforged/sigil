use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

use crate::events::prelude::NodeEventsExt;
use crate::kernel::{
    Cx, IntoNode, NodeBuilder, NodeId, Role, container, node, schedule_keyed_rebuild,
};
use crate::layout::LayoutReactiveExt;
use crate::layout::prelude::{Dimension, LayoutBuilderExt, LayoutStyle, Overflow, Position};
use crate::reactive::{Signal, derived, effect_named, signal as signal_of};

use super::selection::SelectionRuntime;
use super::window::{
    VisibleWindow, page_rows, prefix_row_top, row_top, strip_height, total_height, visible_window,
};

pub(super) type HeightSpec<'a> = (f32, Option<&'a [f32]>);

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

pub(super) struct BoundStrip<T, R> {
    pub render: Rc<R>,
    pub row_height: f32,
    pub viewport_height: f32,
    pub overscan: usize,
    pub width: WidthSpec,
    pub selection: SelectionRuntime,
    pub live_len: Signal<usize>,
    pub row_key: Option<super::RowKey<T>>,
}

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
    let overscan = spec.overscan;
    let height = strip_height(vh, rh, overscan);
    let mounted = mounted_window(signal, scroll_y.get(), &spec);
    let anchor = signal_of(mounted.first);
    let pos = derived(move || strip_offset(anchor.get(), scroll_y.get(), rh));
    let first = keyed_rows(signal, mounted, &spec, 0, None);
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
        let runner = Strip {
            parent,
            prior,
            primed: Cell::new(true),
            last_len: Cell::new(spec.live_len.peek()),
            generation: Cell::new(0),
            window: Cell::new(mounted),
            anchor,
            scroll_y,
            signal,
            spec,
        };
        effect_named("sigil-virtual-list::strip", move || runner.run());
    })
}

struct Strip<S: 'static, T: 'static, R: 'static> {
    parent: NodeId,
    prior: Rc<MountedRows>,
    primed: Cell<bool>,
    last_len: Cell<usize>,
    generation: Cell<u64>,
    window: Cell<VisibleWindow>,
    anchor: Signal<usize>,
    scroll_y: Signal<f32>,
    signal: Signal<S>,
    spec: BoundStrip<T, R>,
}

impl<S, T, R> Strip<S, T, R>
where
    S: Deref + Clone + 'static,
    S::Target: AsRef<[T]>,
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
{
    fn run(&self) {
        let len = self.spec.live_len.get();
        let y = self.scroll_y.get();
        let mut stale = false;
        if self.spec.row_key.is_some() {
            let _ = self.signal.get();
            stale = true;
        } else if self.last_len.replace(len) != len {
            self.generation.set(self.generation.get() + 1);
            stale = true;
        }
        if self.primed.replace(false) {
            return;
        }
        let next = mounted_window(self.signal, y, &self.spec);
        if !stale && next == self.window.get() {
            return;
        }
        self.window.set(next);
        self.anchor.set(next.first);
        let incoming = keyed_rows(
            self.signal,
            next,
            &self.spec,
            self.generation.get(),
            Some(&self.prior),
        );
        schedule_keyed_rebuild(self.parent, Rc::clone(&self.prior), incoming);
    }
}

fn mounted_window<S, T, R>(
    signal: Signal<S>,
    scroll_y: f32,
    spec: &BoundStrip<T, R>,
) -> VisibleWindow
where
    S: Deref + Clone + 'static,
    S::Target: AsRef<[T]>,
{
    let items = signal.peek();
    let slice: &[T] = (*items).as_ref();
    let window = visible_window(
        scroll_y,
        slice.len(),
        spec.row_height,
        spec.viewport_height,
        spec.overscan,
    );
    let block = mount_block(spec.viewport_height, spec.row_height);
    VisibleWindow {
        first: window.first / block * block,
        end: window
            .end
            .div_ceil(block)
            .saturating_mul(block)
            .min(slice.len()),
    }
}

const MOUNT_BLOCKS_PER_PAGE: usize = 4;

fn mount_block(viewport_height: f32, row_height: f32) -> usize {
    (page_rows(viewport_height, row_height) / MOUNT_BLOCKS_PER_PAGE).max(1)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "row indices are far below the f32 mantissa limit"
)]
fn strip_offset(first: usize, scroll_y: f32, row_height: f32) -> (f32, f32) {
    (0.0, (first as f32).mul_add(row_height, -scroll_y))
}

fn keyed_rows<S, T, R>(
    signal: Signal<S>,
    window: VisibleWindow,
    spec: &BoundStrip<T, R>,
    generation: u64,
    mounted: Option<&MountedRows>,
) -> Vec<(u64, Box<dyn IntoNode>)>
where
    S: Deref + Clone + 'static,
    S::Target: AsRef<[T]>,
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
{
    let items = signal.peek();
    let slice: &[T] = (*items).as_ref();
    let mut rows = Vec::with_capacity(window.len());
    for index in window.first..window.end {
        let Some(item) = slice.get(index) else {
            break;
        };
        let key = spec.row_key.as_ref().map_or_else(
            || (generation << 32) | index as u64,
            |key_of| key_of(index, item),
        );
        let kept = mounted.is_some_and(|mounted| mounted.borrow().iter().any(|(k, _)| *k == key));
        if kept {
            rows.push((key, node(container())));
            continue;
        }
        rows.push((key, node(deferred_row(items.clone(), index, spec))));
    }
    rows
}

fn deferred_row<S, T, R>(
    items: S,
    index: usize,
    spec: &BoundStrip<T, R>,
) -> impl FnOnce(&mut Cx) -> NodeId + use<S, T, R>
where
    S: Deref + 'static,
    S::Target: AsRef<[T]>,
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
{
    let render = Rc::clone(&spec.render);
    let row_height = spec.row_height;
    let width = spec.width;
    let selection = spec.selection;
    move |cx: &mut Cx| {
        let slice: &[T] = (*items).as_ref();
        let row = slice
            .get(index)
            .map_or_else(container, |item| (*render)(index, item));
        place_row(index, row_height, width, row, selection).build(cx)
    }
}
