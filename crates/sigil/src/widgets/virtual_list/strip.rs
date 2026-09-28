use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, container, node, schedule_keyed_rebuild};
use crate::layout::LayoutReactiveExt;
use crate::layout::prelude::{LayoutBuilderExt, LayoutStyle, Position};
use crate::reactive::{Signal, derived, effect_named, signal as signal_of};

use super::measure::{Measure, Tops, row_key_of};
use super::render::{WidthSpec, place_row, with_selection};
use super::selection::SelectionRuntime;
use super::window::{RowHeights, VisibleWindow, page_rows, strip_height, visible_window};

type MountedRows = RefCell<Vec<(u64, NodeId)>>;

pub(super) struct BoundStrip<T, R> {
    pub render: Rc<R>,
    pub row_height: f32,
    pub viewport_height: f32,
    pub overscan: usize,
    pub width: WidthSpec,
    pub selection: SelectionRuntime,
    pub row_key: Option<super::RowKey<T>>,
    pub measure: Option<Measure>,
    pub tops: Option<Tops>,
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
    let mounted = mounted_window(signal, scroll_y.get(), &spec);
    let content = revision(signal);
    let anchor = signal_of(mounted.first);
    let pos = strip_position(anchor, scroll_y, &spec);
    let first = keyed_rows(signal, mounted, &spec, 0, None);
    let seed: Vec<u64> = first.iter().map(|(key, _)| *key).collect();
    let mut strip = container()
        .layout(strip_layout(&spec))
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
            seen: Cell::new(content.peek()),
            content,
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

fn strip_layout<T, R>(spec: &BoundStrip<T, R>) -> LayoutStyle {
    let style = LayoutStyle::block()
        .position(Position::Absolute)
        .inset_left(0.0);
    let style = if spec.tops.is_some() {
        style
    } else {
        style.height(strip_height(
            spec.viewport_height,
            spec.row_height,
            spec.overscan,
        ))
    };
    super::render::width_apply(spec.width, style)
}

fn strip_position<T, R>(
    anchor: Signal<usize>,
    scroll_y: Signal<f32>,
    spec: &BoundStrip<T, R>,
) -> Signal<(f32, f32)> {
    let rh = spec.row_height;
    let tops = spec.tops;
    derived(move || {
        let first = anchor.get();
        let top = tops.map_or_else(
            || RowHeights::fixed(first, rh).total(),
            |tops| RowHeights::Tops(&tops.get()).top(first),
        );
        (0.0, top - scroll_y.get())
    })
}

fn revision<S: Clone + 'static>(signal: Signal<S>) -> Signal<u64> {
    let mut revision = 0_u64;
    derived(move || {
        drop(signal.get());
        revision += 1;
        revision
    })
}

struct Strip<S: 'static, T: 'static, R: 'static> {
    parent: NodeId,
    prior: Rc<MountedRows>,
    primed: Cell<bool>,
    seen: Cell<u64>,
    content: Signal<u64>,
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
        let y = self.scroll_y.get();
        let content = self.content.get();
        if let Some(tops) = self.spec.tops {
            tops.track();
        }
        let stale = self.seen.replace(content) != content;
        if stale {
            self.generation.set(self.generation.get() + 1);
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
    let window = spec.tops.map_or_else(
        || {
            let heights = RowHeights::fixed(slice.len(), spec.row_height);
            visible_window(scroll_y, heights, spec.viewport_height, spec.overscan)
        },
        |tops| {
            let tops = tops.peek();
            visible_window(
                scroll_y,
                RowHeights::Tops(&tops),
                spec.viewport_height,
                spec.overscan,
            )
        },
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
        let cache_key = row_key_of(spec.row_key.as_ref(), index, item);
        rows.push((
            key,
            node(deferred_row(items.clone(), index, cache_key, spec)),
        ));
    }
    rows
}

fn deferred_row<S, T, R>(
    items: S,
    index: usize,
    cache_key: u64,
    spec: &BoundStrip<T, R>,
) -> impl FnOnce(&mut Cx) -> NodeId + use<S, T, R>
where
    S: Deref + 'static,
    S::Target: AsRef<[T]>,
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
{
    let render = Rc::clone(&spec.render);
    let row_height = spec.tops.map_or(spec.row_height, |tops| {
        RowHeights::Tops(&tops.peek()).span(index).1
    });
    let width = spec.width;
    let selection = spec.selection;
    let measure = spec.measure.clone();
    move |cx: &mut Cx| {
        let slice: &[T] = (*items).as_ref();
        let row = slice
            .get(index)
            .map_or_else(container, |item| (*render)(index, item));
        let slot = match measure {
            Some(measure) => with_selection(measure.slot(cache_key, width, row), index, selection),
            None => place_row(index, row_height, width, row, selection),
        };
        slot.build(cx)
    }
}
