#![allow(clippy::redundant_pub_crate, dead_code)]

use std::cell::RefCell;
use std::rc::Rc;

use sigil_events::prelude::*;
use sigil_hal::{KeyEventInit, Point, PointerEventInit, Shell};
use sigil_kernel::prelude::*;
use sigil_layout::prelude::*;

use super::mock_shell::MockShell;

pub(crate) fn route_event(
    router: &mut EventRouter,
    cx: &mut Cx,
    layout: &LayoutEngine,
    input_event: ShellEvent,
) {
    let mut shell = MockShell::new();
    shell.push_event(input_event);
    for shell_event in shell.drain_events() {
        router.route(cx, layout, shell_event);
    }
}

pub(crate) fn compute_layout(cx: &mut Cx, root: NodeId, width: f32, height: f32) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(ZeroTextMeasure));
    layout
        .compute(cx, root, Size::fixed(width, height))
        .unwrap();
    layout
}

pub(crate) const fn pointer_down(x: f32, y: f32) -> ShellEvent {
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

pub(crate) const fn pointer_move(x: f32, y: f32) -> ShellEvent {
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

pub(crate) fn tab(shift: bool) -> ShellEvent {
    let mut modifiers = ModifiersState::empty();
    modifiers.set(ModifiersState::SHIFT, shift);
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: Key::Tab.into(),
        physical: PhysicalKey::Tab,
        modifiers,
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

pub(crate) fn ctrl_s() -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character("s"),
        physical: PhysicalKey::KeyS,
        modifiers: ModifiersState::CONTROL,
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

pub(crate) fn key_event(key: Key, physical: PhysicalKey, phase: KeyPhase) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: key.into(),
        physical,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase,
        text: None,
    }))
}

pub(crate) const fn window_blur() -> ShellEvent {
    ShellEvent::Window(sigil_hal::WindowEvent::Focused(false))
}

pub(crate) fn abs_rect(x: f32, y: f32, width: f32, height: f32) -> LayoutStyle {
    LayoutStyle::block()
        .fixed_size(width, height)
        .position(Position::Absolute)
        .with_taffy(move |style| {
            style.inset.left = LengthPercentageAuto::length(x);
            style.inset.top = LengthPercentageAuto::length(y);
        })
}

pub(crate) fn record(
    order: &Rc<RefCell<Vec<&'static str>>>,
    value: &'static str,
) -> impl Fn(&mut PointerEvent) + 'static {
    let order = Rc::clone(order);
    move |_| order.borrow_mut().push(value)
}

pub(crate) fn record_focus(
    order: &Rc<RefCell<Vec<&'static str>>>,
    value: &'static str,
) -> impl Fn() + 'static {
    let order = Rc::clone(order);
    move || order.borrow_mut().push(value)
}

pub(crate) fn focus_tree(tab_indexes: [Option<i32>; 3]) -> (Cx, NodeId, [NodeId; 3]) {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .child(focusable_child(tab_indexes[0]))
            .child(focusable_child(tab_indexes[1]))
            .child(focusable_child(tab_indexes[2])),
    );
    let ids = [
        cx.nth_child(root, 0).unwrap(),
        cx.nth_child(root, 1).unwrap(),
        cx.nth_child(root, 2).unwrap(),
    ];
    (cx, root, ids)
}

pub(crate) fn focusable_child(tab_index: Option<i32>) -> NodeBuilder {
    let builder = container()
        .focusable(true)
        .layout(LayoutStyle::block().fixed_size(10.0, 10.0));
    if let Some(index) = tab_index {
        builder.tab_index(index)
    } else {
        builder
    }
}

pub(crate) struct ZeroTextMeasure;

impl TextMeasure for ZeroTextMeasure {
    fn measure(
        &self,
        _content: &TextContent,
        _space: taffy::prelude::Size<taffy::prelude::AvailableSpace>,
    ) -> taffy::prelude::Size<f32> {
        taffy::prelude::Size::ZERO
    }
}
