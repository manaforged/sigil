#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::too_many_lines)]
#![allow(clippy::shadow_unrelated)]

#[path = "widget_support/mod.rs"]
mod widget_support;

use std::cell::RefCell;
use std::rc::Rc;

use sigil::hal::{Point, PointerEventInit};
use sigil::prelude::*;
use sigil::scene::{SceneCommand, emit_scene};
use widget_support::{compute, rect_colors};

const WIDTH: f32 = 640.0;
const HEIGHT: f32 = 480.0;
const VIEWPORT: Size = Size::fixed(WIDTH, HEIGHT);

fn glyph_texts(cx: &Cx, root: NodeId) -> Vec<String> {
    emit_scene(cx, root)
        .commands
        .iter()
        .filter_map(|c| match c {
            SceneCommand::GlyphRun(d) => Some(d.run.text.clone()),
            _ => None,
        })
        .collect()
}

const fn pointer(phase: PointerPhase, x: f32, y: f32) -> ShellEvent {
    let (button, buttons) = match phase {
        PointerPhase::Down => (Some(MouseButton::Left), MouseButtons::LEFT),
        PointerPhase::Move => (None, MouseButtons::LEFT),
        _ => (None, MouseButtons::empty()),
    };
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button,
        buttons,
        modifiers: ModifiersState::empty(),
        phase,
    }))
}

const fn right_down(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Right),
        buttons: MouseButtons::RIGHT,
        modifiers: ModifiersState::empty(),
        phase: PointerPhase::Down,
    }))
}

fn route(router: &mut EventRouter, cx: &mut Cx, layout: &LayoutEngine, event: ShellEvent) {
    router.route(cx, layout, event);
}

type Drops = Rc<RefCell<Vec<String>>>;
type Downs = Rc<RefCell<u32>>;

fn demo_tree(
    state: DragState<String>,
    drops: &Drops,
    downs: &Downs,
    accepts: impl Fn(&String) -> bool + 'static,
) -> impl IntoNode {
    let on_drop = {
        let drops = Rc::clone(drops);
        move |payload: String| drops.borrow_mut().push(payload)
    };
    let count_down = {
        let downs = Rc::clone(downs);
        move |_: &mut sigil::hal::PointerEvent| *downs.borrow_mut() += 1
    };
    container()
        .layout(LayoutStyle::flex_row().fixed_size(WIDTH, HEIGHT))
        .child(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(100.0, 40.0))
                .drag_source(state.clone(), "task-7".to_owned())
                .pointer_down(count_down)
                .child(text("task-7")),
        )
        .child(container().layout(LayoutStyle::flex_column().fixed_size(100.0, 40.0)))
        .child(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(100.0, 40.0))
                .drop_target(state.clone(), accepts, on_drop),
        )
        .child(drag_ghost(state, |payload| {
            text(format!("moving {payload}"))
        }))
}

fn demo() -> (DragState<String>, Drops, Downs, Cx, NodeId) {
    let state = DragState::<String>::new();
    let drops: Drops = Rc::new(RefCell::new(Vec::new()));
    let downs: Downs = Rc::new(RefCell::new(0_u32));
    let mut cx = Cx::new_root();
    let root = cx.add(demo_tree(state.clone(), &drops, &downs, |_| true));
    (state, drops, downs, cx, root)
}

#[path = "drag_drop/session.rs"]
mod session;
#[path = "drag_drop/targets.rs"]
mod targets;
