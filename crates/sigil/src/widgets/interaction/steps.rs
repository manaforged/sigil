use crate::events::{Key, KeyEvent, NodeEventsExt};
use crate::kernel::NodeBuilder;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Step {
    Unit(f32),
    Page(f32),
    Start,
    End,
}

pub(crate) fn step_keys(
    node: NodeBuilder,
    keys: &[(Key, Step)],
    on_step: impl Fn(Step, &mut KeyEvent) + Clone + 'static,
) -> NodeBuilder {
    keys.iter().fold(node, |node, &(key, step)| {
        let on_step = on_step.clone();
        node.key(key, move |event| on_step(step, event))
    })
}
