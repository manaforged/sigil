#![allow(clippy::redundant_pub_crate, dead_code)]

use sigil_kittest::FixedMeasure;
use std::cell::RefCell;
use std::rc::Rc;

use sigil::events::prelude::*;
use sigil::hal::Shell;
use sigil::kernel::prelude::*;
use sigil::layout::prelude::*;

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
    let mut layout = LayoutEngine::with_text_measure(Box::new(FixedMeasure::default()));
    layout
        .compute(cx, root, Size::fixed(width, height))
        .unwrap();
    layout
}

pub(crate) fn abs_rect(x: f32, y: f32, width: f32, height: f32) -> LayoutStyle {
    LayoutStyle::block()
        .fixed_size(width, height)
        .position(Position::Absolute)
        .with_layout(move |style| {
            style.inset.left = LengthPercentageAuto::Px(x);
            style.inset.top = LengthPercentageAuto::Px(y);
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
