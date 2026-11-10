#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::panic)]

use std::cell::Cell;
use std::rc::Rc;

use criterion::{Criterion, criterion_group, criterion_main};
use sigil::events::{
    Event, EventRouter, Key, KeyPhase, LogicalKey, ModifiersState, MouseButton, MouseButtons,
    PhysicalKey, PointerEvent, PointerPhase, Shortcut,
};
use sigil::geometry::{Point, Size};
use sigil::kernel::{Cx, NodeBuilder, NodeId, TextContent, container};
use sigil::layout::{LayoutEngine, LayoutStyle, Length, Position, TextMeasure};
use sigil::prelude::Widget;
use sigil::style::TextStyle;
use sigil::widgets::Layout;

fn hit_test_1000_nodes(c: &mut Criterion) {
    let (mut cx, _root, layout) = wide_tree(1_000, false);
    let mut router = EventRouter::new();
    c.bench_function("hit_test_1000_nodes", |b| {
        b.iter(|| router.route(&mut cx, &layout, pointer_down(0.5, 0.5)));
    });
}

fn route_pointer_event_depth_20(c: &mut Criterion) {
    let (mut cx, _root, layout) = deep_tree(20);
    let mut router = EventRouter::new();
    c.bench_function("route_pointer_event_depth_20", |b| {
        b.iter(|| router.route(&mut cx, &layout, pointer_down(0.5, 0.5)));
    });
}

fn route_pointer_event_depth_64(c: &mut Criterion) {
    let (mut cx, _root, layout) = deep_tree(64);
    let mut router = EventRouter::new();
    c.bench_function("route_pointer_event_depth_64", |b| {
        b.iter(|| router.route(&mut cx, &layout, pointer_down(0.5, 0.5)));
    });
}

fn route_captured_pointer_event(c: &mut Criterion) {
    let (mut cx, _root, layout) = deep_tree_with_leaf(
        32,
        container()
            .layout(LayoutStyle::block().fixed_size(10.0, 10.0))
            .pointer_down(PointerEvent::capture_pointer)
            .pointer_move(|_| {}),
    );
    let mut router = EventRouter::new();
    router.route(&mut cx, &layout, pointer_down(0.5, 0.5));
    c.bench_function("route_captured_pointer_event", |b| {
        b.iter(|| router.route(&mut cx, &layout, pointer_move(200.0, 200.0)));
    });
}

fn route_captured_pointer_release(c: &mut Criterion) {
    let (mut cx, _root, layout) = deep_tree_with_leaf(
        32,
        container()
            .layout(LayoutStyle::block().fixed_size(10.0, 10.0))
            .pointer_down(PointerEvent::capture_pointer)
            .pointer_up(|_| {}),
    );
    let mut router = EventRouter::new();
    c.bench_function("route_captured_pointer_release", |b| {
        b.iter(|| {
            router.route(&mut cx, &layout, pointer_down(0.5, 0.5));
            router.route(&mut cx, &layout, pointer_up(200.0, 200.0));
        });
    });
}

fn route_hover_transition_depth_20(c: &mut Criterion) {
    let (mut cx, _root, layout) = deep_tree_with_leaf(
        20,
        container()
            .layout(LayoutStyle::block().fixed_size(10.0, 10.0))
            .pointer_enter(|_| {})
            .pointer_leave(|_| {}),
    );
    let mut router = EventRouter::new();
    let mut inside = false;
    c.bench_function("route_hover_transition_depth_20", |b| {
        b.iter(|| {
            inside = !inside;
            let event = if inside {
                pointer_move(0.5, 0.5)
            } else {
                pointer_move(200.0, 200.0)
            };
            router.route(&mut cx, &layout, event);
        });
    });
}

fn hit_test_10_000_nodes(c: &mut Criterion) {
    let (mut cx, _root, layout) = wide_tree(10_000, false);
    let mut router = EventRouter::new();
    c.bench_function("hit_test_10_000_nodes", |b| {
        b.iter(|| router.route(&mut cx, &layout, pointer_down(0.5, 0.5)));
    });
}

fn shortcut_dispatch_1000_nodes(c: &mut Criterion) {
    let hits = Rc::new(Cell::new(0_u32));
    let shortcut = Shortcut::ctrl(LogicalKey::character("s"));
    let (mut cx, _root, layout) = wide_tree_with_shortcut(1_000, &shortcut, &hits);
    let mut router = EventRouter::new();
    c.bench_function("shortcut_dispatch_1000_nodes", |b| {
        b.iter(|| router.route(&mut cx, &layout, ctrl_s()));
    });
}

fn tab_ring_rebuild_1000_focusable(c: &mut Criterion) {
    let (mut cx, _root, layout) = wide_tree(1_000, true);
    c.bench_function("tab_ring_rebuild_1000_focusable", |b| {
        b.iter(|| {
            let mut router = EventRouter::new();
            router.route(&mut cx, &layout, tab());
        });
    });
}

criterion_group!(
    events_ops,
    hit_test_1000_nodes,
    route_pointer_event_depth_20,
    route_pointer_event_depth_64,
    route_captured_pointer_event,
    route_captured_pointer_release,
    route_hover_transition_depth_20,
    hit_test_10_000_nodes,
    shortcut_dispatch_1000_nodes,
    tab_ring_rebuild_1000_focusable
);
criterion_main!(events_ops);

fn wide_tree(count: usize, focusable: bool) -> (Cx, NodeId, LayoutEngine) {
    let mut cx = Cx::new_root();
    let mut root_builder = container().layout(LayoutStyle::block().fixed_size(240.0, 240.0));
    for index in 0..count {
        root_builder = root_builder.child(wide_child(index, focusable));
    }
    let root = cx.add(root_builder);
    let layout = compute_layout(&mut cx, root);
    (cx, root, layout)
}

fn deep_tree(depth: usize) -> (Cx, NodeId, LayoutEngine) {
    deep_tree_with_leaf(
        depth,
        container().layout(LayoutStyle::block().fixed_size(10.0, 10.0)),
    )
}

fn deep_tree_with_leaf(depth: usize, leaf: NodeBuilder) -> (Cx, NodeId, LayoutEngine) {
    let mut cx = Cx::new_root();
    let mut child = leaf;
    for _ in 1..depth {
        child = container()
            .layout(LayoutStyle::block().fixed_size(10.0, 10.0))
            .child(child);
    }
    let root = cx.add(child);
    let layout = compute_layout(&mut cx, root);
    (cx, root, layout)
}

fn wide_tree_with_shortcut(
    count: usize,
    shortcut: &Shortcut,
    hits: &Rc<Cell<u32>>,
) -> (Cx, NodeId, LayoutEngine) {
    let mut cx = Cx::new_root();
    let mut root_builder = container().layout(LayoutStyle::block().fixed_size(240.0, 240.0));
    for index in 0..count {
        let child = if index + 1 == count {
            wide_child(index, false)
                .keyboard_shortcut(shortcut.clone())
                .key(shortcut.clone(), {
                    let hits = Rc::clone(hits);
                    move |_| hits.set(hits.get() + 1)
                })
        } else {
            wide_child(index, false)
        };
        root_builder = root_builder.child(child);
    }
    let root = cx.add(root_builder);
    let layout = compute_layout(&mut cx, root);
    (cx, root, layout)
}

fn compute_layout(cx: &mut Cx, root: NodeId) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(ZeroTextMeasure));
    layout
        .compute(cx, root, Size::fixed(240.0, 240.0))
        .expect("bench fixture layout computes");
    layout
}

fn wide_child(index: usize, focusable: bool) -> NodeBuilder {
    let builder = container().layout(child_style(index)).focusable(focusable);
    if focusable {
        builder.tab_index(index_to_tab(index))
    } else {
        builder
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "pixel/char magnitudes are bounded; sub-unit precision is irrelevant"
)]
fn child_style(index: usize) -> LayoutStyle {
    let x = ((index % 100) * 2) as f32;
    let y = ((index / 100) * 2) as f32;
    abs_rect(x, y, 1.0, 1.0)
}

fn index_to_tab(index: usize) -> i32 {
    i32::try_from(index).unwrap_or(i32::MAX)
}

fn abs_rect(x: f32, y: f32, width: f32, height: f32) -> LayoutStyle {
    LayoutStyle::block()
        .fixed_size(width, height)
        .position(Position::Absolute)
        .with_layout(move |style| {
            style.inset.left = Length::Px(x);
            style.inset.top = Length::Px(y);
        })
}

const fn pointer_down(x: f32, y: f32) -> Event {
    Event::pointer(PointerPhase::Down, Point::new(x, y))
        .button(Some(MouseButton::Left))
        .buttons(MouseButtons::LEFT)
}

const fn pointer_move(x: f32, y: f32) -> Event {
    Event::pointer(PointerPhase::Move, Point::new(x, y)).buttons(MouseButtons::LEFT)
}

const fn pointer_up(x: f32, y: f32) -> Event {
    Event::pointer(PointerPhase::Up, Point::new(x, y)).button(Some(MouseButton::Left))
}

fn tab() -> Event {
    Event::key(Key::Tab, KeyPhase::Down).physical(PhysicalKey::Tab)
}

fn ctrl_s() -> Event {
    Event::key(LogicalKey::character("s"), KeyPhase::Down)
        .physical(PhysicalKey::KeyS)
        .modifiers(ModifiersState::CONTROL)
}

struct ZeroTextMeasure;

impl TextMeasure for ZeroTextMeasure {
    fn measure(
        &self,
        _content: &TextContent,
        _space: sigil::geometry::Size,
    ) -> sigil::geometry::LogicalSize {
        sigil::geometry::LogicalSize::default()
    }

    fn measure_styled(
        &self,
        _content: &TextContent,
        _style: &TextStyle,
        _space: sigil::geometry::Size,
    ) -> sigil::geometry::LogicalSize {
        sigil::geometry::LogicalSize::default()
    }
}
