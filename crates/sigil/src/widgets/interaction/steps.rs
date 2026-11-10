use crate::events::{Key, KeyEvent};
use crate::kernel::NodeBuilder;
use crate::widgets::Widget;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ValueStep {
    Unit(f32),
    Page(f32),
    Start,
    End,
}

pub(crate) fn step_keys(
    node: NodeBuilder,
    keys: &[(Key, ValueStep)],
    on_step: impl Fn(ValueStep, &mut KeyEvent) + Clone + 'static,
) -> NodeBuilder {
    keys.iter().fold(node, |node, &(key, step)| {
        let on_step = on_step.clone();
        node.key(key, move |event| on_step(step, event))
    })
}
