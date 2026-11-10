use std::rc::Rc;

use crate::kernel::NodeBuilder;
use crate::layout::Orientation;
use crate::widgets::interaction::{DragEnd, axis_of, pointer_drag};

/// Drag along one axis.
pub fn axis_drag<S>(
    node: NodeBuilder,
    axis: Orientation,
    snapshot: impl Fn() -> S + 'static,
    on_delta: impl Fn(S, f32) + 'static,
    on_end: impl Fn() + 'static,
) -> NodeBuilder
where
    S: Clone + 'static,
{
    let on_delta = Rc::new(on_delta);
    let restore = Rc::clone(&on_delta);
    pointer_drag(
        node,
        move |event| {
            event.stop_propagation();
            Some((
                axis_of(axis, event.position.x, event.position.y),
                snapshot(),
            ))
        },
        move |(down, base), event| {
            on_delta(
                base.clone(),
                axis_of(axis, event.position.x, event.position.y) - down,
            );
            event.stop_propagation();
        },
        move |ended, event| {
            match ended {
                DragEnd::Release => on_end(),
                DragEnd::Cancel((_, base)) => restore(base, 0.0),
                DragEnd::Idle => {}
            }
            event.stop_propagation();
        },
    )
}
