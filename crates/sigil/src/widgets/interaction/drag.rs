use crate::events::{DragPhase, PointerEvent};
use crate::kernel::NodeBuilder;
use crate::widgets::Widget;

pub(crate) enum DragEnd<S> {
    Release,
    Cancel(S),
}

pub(crate) fn pointer_drag<S: 'static>(
    node: NodeBuilder,
    begin: impl Fn(&mut PointerEvent) -> Option<S> + 'static,
    drag: impl Fn(&S, &mut PointerEvent) + 'static,
    end: impl Fn(DragEnd<S>, &mut PointerEvent) + 'static,
) -> NodeBuilder {
    let mut session = None::<S>;
    node.pointer_drag(move |phase, event| match (phase, session.take()) {
        (DragPhase::Start, _) => {
            session = begin(event);
            if session.is_none() {
                event.prevent_default();
            }
        }
        (DragPhase::Move, Some(state)) => {
            drag(&state, event);
            session = Some(state);
        }
        (DragPhase::End, _) => end(DragEnd::Release, event),
        (DragPhase::Cancel, Some(state)) => end(DragEnd::Cancel(state), event),
        (DragPhase::Move | DragPhase::Cancel, None) => {}
    })
}
