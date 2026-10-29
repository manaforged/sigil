use crate::kernel::NodeBuilder;
use crate::widgets::interaction::{axis_of, pointer_drag};
use crate::widgets::orientation::Orientation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Axis {
    X,
    Y,
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
    let axis = match axis {
        Axis::X => Orientation::Horizontal,
        Axis::Y => Orientation::Vertical,
    };
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
            if ended.is_some() {
                on_end();
            }
            event.stop_propagation();
        },
    )
}
