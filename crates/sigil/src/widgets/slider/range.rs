use std::cell::Cell;
use std::ops::RangeInclusive;
use std::rc::Rc;

use crate::layout::{LayoutStyle, Orientation};
use crate::node::{AccessibleName, NodeBuilder, Role, container};
use crate::reactive::{Signal, memo, signal};
use crate::style::{StringKey, Theme};
use crate::widgets::Widget;

use super::builder::{SliderRuntime, ValueChange, control_frame, slider_layout, slider_semantics};
use super::handlers::{Grip, ThumbSignals, attach_key_handlers, attach_pointer_drag};
use super::thumb::Thumb;
use super::track::{segment, slider_thumb};

pub(super) fn range_root(
    runtime: SliderRuntime,
    range: Signal<RangeInclusive<f32>>,
    label: Option<&str>,
    on_change: &ValueChange,
) -> NodeBuilder {
    let extent = signal(crate::geometry::Size::ZERO);
    let start = runtime.on(Thumb::Start(range));
    let end = runtime.on(Thumb::End(range));
    let (start_node, start_grip) = thumb(start, name(label, Some(StringKey::Minimum)), on_change);
    let (end_node, end_grip) = thumb(end, name(label, Some(StringKey::Maximum)), on_change);
    let mut parts = vec![
        segment(
            runtime,
            extent,
            false,
            Some(Box::new(move || start.ratio())),
        ),
        start_node,
        segment(
            runtime,
            extent,
            true,
            Some(Box::new(move || end.ratio() - start.ratio())),
        ),
        end_node,
        segment(runtime, extent, false, None),
    ];
    if runtime.orientation == Orientation::Vertical {
        parts.reverse();
    }
    let root = control_frame(
        container()
            .role(Role::Group)
            .label(name(label, None))
            .track_size(extent)
            .layout(slider_layout(
                runtime.orientation,
                runtime.length,
                label.is_some(),
            )),
        runtime.disabled,
    );
    let root = parts.into_iter().fold(root, NodeBuilder::child);
    attach_pointer_drag(root, start, [start_grip, end_grip], Rc::clone(on_change))
}

fn thumb(
    runtime: SliderRuntime,
    name: AccessibleName,
    on_change: &ValueChange,
) -> (NodeBuilder, Grip) {
    let slot = Rc::new(Cell::new(None));
    let record = Rc::clone(&slot);
    let signals = ThumbSignals {
        hovered: signal(false),
        focused: signal(false),
        pressed: signal(false),
        dragging: signal(false),
    };
    let node = control_frame(
        slider_semantics(runtime, name)
            .focusable(true)
            .hittable(false)
            .layout(LayoutStyle::block().shrink(0.0))
            .track_focused(signals.focused)
            .added(move |_, id| record.set(Some(id))),
        runtime.disabled,
    );
    let node = node.child(slider_thumb(signals, runtime.disabled, runtime.glass));
    let node = attach_key_handlers(node, runtime, Rc::clone(on_change));
    (node, Grip::focused(signals.dragging, slot))
}

fn name(label: Option<&str>, part: Option<StringKey>) -> AccessibleName {
    let base = label.map(str::to_owned);
    match (base, part) {
        (Some(base), None) => AccessibleName::Static(base),
        (base, part) => {
            let fallback = Theme::label(StringKey::Slider);
            let part = part.map(Theme::label);
            AccessibleName::Derived(memo(move || {
                let base = base.clone().unwrap_or_else(|| fallback.get());
                part.map_or_else(|| base.clone(), |part| format!("{base} {}", part.get()))
            }))
        }
    }
}
