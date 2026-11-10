use std::cell::RefCell;
use std::rc::Rc;

use crate::events::PointerEvent;
use crate::kernel::NodeBuilder;
use crate::widgets::Widget;
use crate::widgets::orientation::Orientation;

use super::{is_cancel, is_primary};

pub(crate) enum DragEnd<S> {
    Release,
    Cancel(S),
    Idle,
}

pub(crate) const fn axis_of(axis: Orientation, x: f32, y: f32) -> f32 {
    match axis {
        Orientation::Horizontal => x,
        Orientation::Vertical => y,
    }
}

pub(crate) fn pointer_drag<S: 'static>(
    node: NodeBuilder,
    begin: impl Fn(&mut PointerEvent) -> Option<S> + 'static,
    drag: impl Fn(&S, &mut PointerEvent) + 'static,
    end: impl Fn(DragEnd<S>, &mut PointerEvent) + 'static,
) -> NodeBuilder {
    let session = Rc::new(RefCell::new(None::<S>));
    let (begin_session, move_session) = (Rc::clone(&session), Rc::clone(&session));
    node.pointer_down(move |event| {
        if !is_primary(event) {
            return;
        }
        if let Some(state) = begin(event) {
            *begin_session.borrow_mut() = Some(state);
            event.capture_pointer();
        }
    })
    .pointer_move(move |event| {
        if !event.is_captured() {
            return;
        }
        if let Some(state) = move_session.borrow().as_ref() {
            drag(state, event);
        }
    })
    .pointer_up(move |event| {
        let cancelled = is_cancel(event);
        if !cancelled && !is_primary(event) {
            return;
        }
        let ended = match (session.borrow_mut().take(), cancelled) {
            (Some(state), true) => DragEnd::Cancel(state),
            (Some(_), false) => DragEnd::Release,
            (None, _) => DragEnd::Idle,
        };
        end(ended, event);
    })
}
