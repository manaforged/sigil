use crate::events::Key;
use crate::kernel::NodeBuilder;
use crate::reactive::derived;
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_label;
use crate::widgets::interaction::{ValueStep, step_keys};

use super::drag::SplitterState;
use crate::layout::Orientation;

pub(in crate::widgets) const KEY_STEP: f32 = 16.0;

pub(super) fn install_handle_keys(
    node: NodeBuilder,
    state: SplitterState,
    handle: usize,
) -> NodeBuilder {
    let (min, max) = state.bounds(handle);
    let tracked = state.clone();
    let value = derived(move || tracked.tracked_size(handle));
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
        state.settle();
    })
}

pub(in crate::widgets) fn resize_keys(
    node: NodeBuilder,
    (back, forward): (Key, Key),
    step_by: impl Fn(f32) + Clone + 'static,
) -> NodeBuilder {
    let keys = [
        (back, ValueStep::Unit(-1.0)),
        (forward, ValueStep::Unit(1.0)),
        (Key::Home, ValueStep::Start),
        (Key::End, ValueStep::End),
    ];
    step_keys(node, &keys, move |step, event| {
        step_by(match step {
            ValueStep::Unit(sign) | ValueStep::Page(sign) => sign * KEY_STEP,
            ValueStep::Start => f32::NEG_INFINITY,
            ValueStep::End => f32::INFINITY,
        });
        event.stop_propagation();
    })
}
