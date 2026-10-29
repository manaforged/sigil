use crate::events::Key;
use crate::kernel::NodeBuilder;
use crate::reactive::derived;
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_label;
use crate::widgets::interaction::{Step, step_keys};

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
    let keys = [
        (back, Step::Unit(-1.0)),
        (forward, Step::Unit(1.0)),
        (Key::Home, Step::Start),
        (Key::End, Step::End),
    ];
    step_keys(node, &keys, move |step, event| {
        step_by(match step {
            Step::Unit(sign) | Step::Page(sign) => sign * KEY_STEP,
            Step::Start => f32::NEG_INFINITY,
            Step::End => f32::INFINITY,
        });
        event.stop_propagation();
    })
}
