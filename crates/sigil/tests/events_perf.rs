#![allow(clippy::panic, clippy::unwrap_used)]

use sigil::geometry::Point;
use sigil::widgets::{Layout, Widget};
use sigil_kittest::FakeMeasure;
use std::cell::Cell;
use std::rc::Rc;

use sigil::events::{Event, Key, KeyPhase, Modifiers, PhysicalKey, PointerEvent, *};
use sigil::geometry::Size;
use sigil::kernel::*;
use sigil::layout::*;

#[test]
fn pointer_route_depth_64_does_not_allocate() {
    let (mut cx, root, layout) = deep_tree(64, None);
    let mut router = EventRouter::new();
    let _ = root;

    let info = allocation_counter::measure(|| {
        router.route(&mut cx, &layout, Event::pointer_down(Point::new(0.5, 0.5)));
    });

    assert_eq!(info.count_total, 0, "{info:?}");
}

#[test]
fn focused_key_route_depth_64_does_not_allocate() {
    let hits = Rc::new(Cell::new(0_u32));
    let (mut cx, root, layout) = deep_tree(
        64,
        Some(
            container()
                .focusable(true)
                .layout(LayoutStyle::block().fixed_size(10.0, 10.0))
                .key(Key::Enter, {
                    let hits = Rc::clone(&hits);
                    move |_| hits.set(hits.get() + 1)
                }),
        ),
    );
    let leaf = deepest_child(&cx, root);
    let mut router = EventRouter::new();
    router.set_focus(&mut cx, Some(leaf));

    let info = allocation_counter::measure(|| {
        router.route(
            &mut cx,
            &layout,
            Event::key_down(Key::Enter).physical(PhysicalKey::Enter),
        );
    });

    assert_eq!(info.count_total, 0, "{info:?}");
    assert_eq!(hits.get(), 1);
}

#[test]
fn captured_pointer_up_release_does_not_allocate() {
    let ups = Rc::new(Cell::new(0_u32));
    let (mut cx, _root, layout) = deep_tree(
        32,
        Some(
            container()
                .layout(LayoutStyle::block().fixed_size(10.0, 10.0))
                .pointer_down(PointerEvent::capture_pointer)
                .pointer_up({
                    let ups = Rc::clone(&ups);
                    move |event| {
                        if event.is_captured() {
                            ups.set(ups.get() + 1);
                        }
                    }
                }),
        ),
    );
    let mut router = EventRouter::new();
    router.route(&mut cx, &layout, Event::pointer_down(Point::new(0.5, 0.5)));

    let info = allocation_counter::measure(|| {
        router.route(
            &mut cx,
            &layout,
            Event::pointer_up(Point::new(200.0, 200.0)),
        );
    });

    assert_eq!(info.count_total, 0, "{info:?}");
    assert_eq!(ups.get(), 1);

    router.route(
        &mut cx,
        &layout,
        Event::pointer_drag(Point::new(200.0, 200.0)),
    );
    assert_eq!(ups.get(), 1);
}

#[test]
fn captured_pointer_move_does_not_allocate() {
    let moves = Rc::new(Cell::new(0_u32));
    let (mut cx, _root, layout) = deep_tree(
        32,
        Some(
            container()
                .layout(LayoutStyle::block().fixed_size(10.0, 10.0))
                .pointer_down(PointerEvent::capture_pointer)
                .pointer_move({
                    let moves = Rc::clone(&moves);
                    move |event| {
                        if event.is_captured() {
                            moves.set(moves.get() + 1);
                        }
                    }
                }),
        ),
    );
    let mut router = EventRouter::new();
    router.route(&mut cx, &layout, Event::pointer_down(Point::new(0.5, 0.5)));

    let info = allocation_counter::measure(|| {
        router.route(
            &mut cx,
            &layout,
            Event::pointer_drag(Point::new(200.0, 200.0)),
        );
    });

    assert_eq!(info.count_total, 0, "{info:?}");
    assert_eq!(moves.get(), 1);
}

#[test]
fn semantic_shortcut_scan_1000_nodes_does_not_allocate() {
    let hits = Rc::new(Cell::new(0_u32));
    let shortcut = Shortcut::ctrl(LogicalKey::character("s"));
    let mut cx = Cx::new_root();
    let mut root_builder = container().layout(LayoutStyle::block().fixed_size(240.0, 240.0));
    for index in 0..1_000 {
        let child = if index == 999 {
            wide_child(index)
                .keyboard_shortcut(shortcut.clone())
                .key(shortcut.clone(), {
                    let hits = Rc::clone(&hits);
                    move |_| hits.set(hits.get() + 1)
                })
        } else {
            wide_child(index)
        };
        root_builder = root_builder.child(child);
    }
    let root = cx.add(root_builder);
    let layout = compute_layout(&mut cx, root);
    let mut router = EventRouter::new();

    router.route(&mut cx, &layout, ctrl_s());
    let event = ctrl_s();
    let info = allocation_counter::measure(|| {
        router.route(&mut cx, &layout, event);
    });

    assert_eq!(info.count_total, 0, "{info:?}");
    assert_eq!(hits.get(), 2);
}

fn deep_tree(depth: usize, leaf: Option<NodeBuilder>) -> (Cx, NodeId, LayoutEngine) {
    let mut cx = Cx::new_root();
    let mut child =
        leaf.unwrap_or_else(|| container().layout(LayoutStyle::block().fixed_size(10.0, 10.0)));
    for _ in 1..depth {
        child = container()
            .layout(LayoutStyle::block().fixed_size(10.0, 10.0))
            .child(child);
    }
    let root = cx.add(child);
    let layout = compute_layout(&mut cx, root);
    (cx, root, layout)
}

fn deepest_child(cx: &Cx, root: NodeId) -> NodeId {
    let mut current = root;
    while let Some(child) = cx.first_child(current) {
        current = child;
    }
    current
}

fn compute_layout(cx: &mut Cx, root: NodeId) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(FakeMeasure::default()));
    layout.compute(cx, root, Size::fixed(240.0, 240.0)).unwrap();
    layout
}

fn wide_child(index: usize) -> NodeBuilder {
    container().layout(child_style(index))
}

#[allow(clippy::cast_precision_loss)]
fn child_style(index: usize) -> LayoutStyle {
    let x = ((index % 100) * 2) as f32;
    let y = ((index / 100) * 2) as f32;
    abs_rect(x, y, 1.0, 1.0)
}

fn abs_rect(x: f32, y: f32, width: f32, height: f32) -> LayoutStyle {
    LayoutStyle::block()
        .fixed_size(width, height)
        .position(Position::Absolute)
        .inset_left(x)
        .inset_top(y)
}

fn ctrl_s() -> Event {
    Event::key(LogicalKey::character("s"), KeyPhase::Down)
        .physical(PhysicalKey::KeyS)
        .modifiers(Modifiers::CTRL)
}
