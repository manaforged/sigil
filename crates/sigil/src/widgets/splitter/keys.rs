use crate::events::{NamedKey, ValueStep};
use crate::kernel::NodeBuilder;
use crate::reactive::memo;
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_label;
use crate::widgets::Widget;

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
    let value = memo(move || tracked.tracked_size(handle));
    let (back, forward) = match state.orientation() {
        Orientation::Horizontal => (NamedKey::ArrowLeft, NamedKey::ArrowRight),
        Orientation::Vertical => (NamedKey::ArrowUp, NamedKey::ArrowDown),
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
    (back, forward): (NamedKey, NamedKey),
    step_by: impl Fn(f32) + Clone + 'static,
) -> NodeBuilder {
    let keys = [
        (back, ValueStep::Unit(-1.0)),
        (forward, ValueStep::Unit(1.0)),
        (NamedKey::Home, ValueStep::Start),
        (NamedKey::End, ValueStep::End),
    ];
    node.step_keys(keys, move |step, event| {
        step_by(match step {
            ValueStep::Unit(sign) | ValueStep::Page(sign) => sign * KEY_STEP,
            ValueStep::Start => f32::NEG_INFINITY,
            ValueStep::End => f32::INFINITY,
        });
        event.stop_propagation();
    })
}
