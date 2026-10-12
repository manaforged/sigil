use std::cell::RefCell;
use std::rc::Rc;

use crate::events::NodeEventsExt;
use crate::kernel::NodeBuilder;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Axis {
    X,
    Y,
}

const fn axis_of(axis: Axis, x: f32, y: f32) -> f32 {
    match axis {
        Axis::X => x,
        Axis::Y => y,
    }
}

pub fn axis_drag<S>(
    node: NodeBuilder,
    axis: Axis,
    snapshot: impl Fn() -> S + 'static,
    on_delta: impl Fn(S, f32) + 'static,
    on_end: impl Fn() + 'static,
) -> NodeBuilder
where
    S: Clone + 'static,
{
    let session = Rc::new(RefCell::new(None::<(f32, S)>));
    let (begin_state, move_state) = (Rc::clone(&session), Rc::clone(&session));
    node.pointer_down(move |event| {
        let axis_pos = axis_of(axis, event.position.x, event.position.y);
        *begin_state.borrow_mut() = Some((axis_pos, snapshot()));
        event.capture_pointer();
        event.stop_propagation();
    })
    .pointer_move(move |event| {
        if !event.is_captured() {
            return;
        }
        let Some((down_axis, base)) = move_state.borrow().clone() else {
            return;
        };
        let current = axis_of(axis, event.position.x, event.position.y);
        on_delta(base, current - down_axis);
        event.stop_propagation();
    })
    .pointer_up(move |event| {
        let ended = session.borrow_mut().take();
        if ended.is_some() {
            on_end();
        }
        event.stop_propagation();
    })
}
