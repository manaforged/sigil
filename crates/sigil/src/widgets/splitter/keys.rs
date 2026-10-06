use crate::events::{Key, NodeEventsExt};
use crate::kernel::NodeBuilder;
use crate::reactive::derived;
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_label;

use super::drag::SplitterState;
use crate::widgets::orientation::Orientation;

pub(in crate::widgets) const KEY_STEP: f32 = 16.0;

pub(super) fn install_handle_keys(
    node: NodeBuilder,
    state: SplitterState,
    handle: usize,
) -> NodeBuilder {
    let (min, max) = state.bounds(handle);
    let value = derived(move || state.tracked_size(handle));
    let (back, forward) = match state.orientation() {
        Orientation::Horizontal => (Key::ArrowLeft, Key::ArrowRight),
        Orientation::Vertical => (Key::ArrowUp, Key::ArrowDown),
    };
    let node = node
        .focusable(true)
        .label(theme_label(StringKey::ResizePanes))
        .value(value, min, max)
        .value_step(KEY_STEP);
    resize_keys(node, (back, forward), move |step| {
        let (low, high) = state.bounds(handle);
        let current = state.size_at(handle);
        let target = (current + step).clamp(low, high);
        state.nudge(handle, target - current);
    })
}

pub(in crate::widgets) fn resize_keys(
    node: NodeBuilder,
    (back, forward): (Key, Key),
    step_by: impl Fn(f32) + Clone + 'static,
) -> NodeBuilder {
    [
        (back, -KEY_STEP),
        (forward, KEY_STEP),
        (Key::Home, f32::NEG_INFINITY),
        (Key::End, f32::INFINITY),
    ]
    .into_iter()
    .fold(node, |node, (key, step)| {
        let step_by = step_by.clone();
        node.key(key, move |event| {
            step_by(step);
            event.stop_propagation();
        })
    })
}
