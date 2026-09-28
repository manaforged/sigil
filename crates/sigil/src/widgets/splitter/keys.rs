use crate::events::prelude::{Key, NodeEventsExt};
use crate::kernel::NodeBuilder;
use crate::reactive::derived;

use super::builder::SplitterOrientation;
use super::drag::SplitterState;

const KEY_STEP: f32 = 16.0;

pub(super) fn install_handle_keys(
    node: NodeBuilder,
    state: SplitterState,
    handle: usize,
) -> NodeBuilder {
    let (min, max) = state.bounds(handle);
    let value = derived(move || state.tracked_size(handle));
    let (back, forward) = match state.orientation() {
        SplitterOrientation::Horizontal => (Key::ArrowLeft, Key::ArrowRight),
        SplitterOrientation::Vertical => (Key::ArrowUp, Key::ArrowDown),
    };
    let node = node
        .focusable(true)
        .label("Resize panes")
        .value(value, min, max)
        .value_step(KEY_STEP);
    [
        (back, -KEY_STEP),
        (forward, KEY_STEP),
        (Key::Home, f32::NEG_INFINITY),
        (Key::End, f32::INFINITY),
    ]
    .into_iter()
    .fold(node, |node, (key, step)| {
        node.key(key, move |event| {
            let (low, high) = state.bounds(handle);
            let current = state.size_at(handle);
            let target = (current + step).clamp(low, high);
            state.nudge(handle, target - current);
            event.stop_propagation();
        })
    })
}
