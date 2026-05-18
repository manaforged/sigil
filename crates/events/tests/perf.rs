#![allow(missing_docs)]
// Performance contract tests keep fixture setup direct. See the
// hygiene framework for the test-only panic/indexing carve-out.
#![allow(clippy::panic, clippy::unwrap_used)]

use std::cell::Cell;
use std::rc::Rc;

use accesskit as _;
use criterion as _;
use sigil_reactive as _;
use sigil_style as _;
use smallvec as _;

use sigil_events::prelude::*;
use sigil_hal::{
    Key, KeyEvent, KeyEventInit, KeyPhase, ModifiersState, MouseButton, MouseButtons, PhysicalKey,
    Point, PointerEvent, PointerEventInit, PointerKind, PointerPhase, ShellEvent,
};
use sigil_kernel::prelude::*;
use sigil_layout::prelude::*;

#[test]
fn pointer_route_depth_64_does_not_allocate() {
    let (mut cx, root, layout) = deep_tree(64, None);
    let mut router = EventRouter::new();
    let _ = root;

    let info = allocation_counter::measure(|| {
        router.route(&mut cx, &layout, pointer_down(0.5, 0.5));
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
                .on_key(Key::Enter, {
                    let hits = Rc::clone(&hits);
                    move |_| hits.set(hits.get() + 1)
                }),
        ),
    );
    let leaf = deepest_child(&cx, root);
    let mut router = EventRouter::new();
    router.set_focus(&mut cx, Some(leaf));

    let info = allocation_counter::measure(|| {
        router.route(&mut cx, &layout, key_down(Key::Enter, PhysicalKey::Enter));
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
                .on_pointer_down(PointerEvent::capture_pointer)
                .on_pointer_up({
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
    router.route(&mut cx, &layout, pointer_down(0.5, 0.5));

    let info = allocation_counter::measure(|| {
        router.route(&mut cx, &layout, pointer_up(200.0, 200.0));
    });

    assert_eq!(info.count_total, 0, "{info:?}");
    assert_eq!(ups.get(), 1);

    // Capture must be released — a follow-up move should take the hit-test
    // path, not the captured fast-path. No allocation contract here; we only
    // prove the capture state was cleared so `up` fires exactly once.
    router.route(&mut cx, &layout, pointer_move(200.0, 200.0));
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
                .on_pointer_down(PointerEvent::capture_pointer)
                .on_pointer_move({
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
    router.route(&mut cx, &layout, pointer_down(0.5, 0.5));

    let info = allocation_counter::measure(|| {
        router.route(&mut cx, &layout, pointer_move(200.0, 200.0));
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
                .on_key(shortcut.clone(), {
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

    let info = allocation_counter::measure(|| {
        router.route(&mut cx, &layout, ctrl_s());
    });

    assert_eq!(info.count_total, 0, "{info:?}");
    assert_eq!(hits.get(), 1);
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
    let mut layout = LayoutEngine::with_text_measure(Box::new(ZeroTextMeasure));
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
        .with_layout(move |style| {
            style.inset.left = LengthPercentageAuto::Px(x);
            style.inset.top = LengthPercentageAuto::Px(y);
        })
}

const fn pointer_down(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        buttons: MouseButtons::LEFT,
        modifiers: ModifiersState::empty(),
        phase: PointerPhase::Down,
    }))
}

const fn pointer_move(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: None,
        buttons: MouseButtons::LEFT,
        modifiers: ModifiersState::empty(),
        phase: PointerPhase::Move,
    }))
}

const fn pointer_up(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        buttons: MouseButtons::empty(),
        modifiers: ModifiersState::empty(),
        phase: PointerPhase::Up,
    }))
}

fn key_down(key: Key, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: key.into(),
        physical,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn ctrl_s() -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character("s"),
        physical: PhysicalKey::KeyS,
        modifiers: ModifiersState::CONTROL,
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

struct ZeroTextMeasure;

impl TextMeasure for ZeroTextMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::default()
    }
}
