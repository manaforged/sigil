use crate::events::DragPhase;
use crate::kernel::NodeBuilder;
use crate::layout::Orientation;
use crate::widgets::Widget;

/// Drag along one axis.
pub fn axis_drag<S>(
    node: impl Into<NodeBuilder>,
    axis: Orientation,
    snapshot: impl Fn() -> S + 'static,
    on_delta: impl Fn(S, f32) + 'static,
    on_end: impl Fn() + 'static,
) -> NodeBuilder
where
    S: Clone + 'static,
{
    let mut session = None::<(f32, S)>;
    node.into().pointer_drag(move |phase, event| {
        let at = match axis {
            Orientation::Horizontal => event.position.x,
            Orientation::Vertical => event.position.y,
        };
        match phase {
            DragPhase::Start => session = Some((at, snapshot())),
            DragPhase::Move => {
                if let Some((down, base)) = &session {
                    on_delta(base.clone(), at - down);
                }
            }
            DragPhase::End => {
                session = None;
                on_end();
            }
            DragPhase::Cancel => {
                if let Some((_, base)) = session.take() {
                    on_delta(base, 0.0);
                }
            }
        }
        event.stop_propagation();
    })
}
