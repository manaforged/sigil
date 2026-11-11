use std::cell::Cell;
use std::ops::RangeInclusive;
use std::rc::Rc;

use crate::kernel::{AccessibleName, NodeBuilder, Role, container};
use crate::layout::{LayoutStyle, Orientation};
use crate::reactive::{Signal, derived};
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_label;
use crate::widgets::Layout;

use super::super::shared::control;
use super::builder::{SliderRuntime, ValueChange, slider_layout, slider_semantics};
use super::handlers::{Grip, attach_key_handlers, attach_pointer_drag};
use super::thumb::Thumb;
use super::track::{segment, slider_thumb};

pub(super) fn range_root(
    runtime: SliderRuntime,
    range: Signal<RangeInclusive<f32>>,
    label: Option<&str>,
    on_change: &ValueChange,
) -> NodeBuilder {
    let start = runtime.on(Thumb::Start(range));
    let end = runtime.on(Thumb::End(range));
    let (start_node, start_grip) = thumb(start, name(label, Some(StringKey::Minimum)), on_change);
    let (end_node, end_grip) = thumb(end, name(label, Some(StringKey::Maximum)), on_change);
    let mut parts = vec![
        segment(runtime, false, Some(Box::new(move || start.ratio()))),
        start_node,
        segment(
            runtime,
            true,
            Some(Box::new(move || end.ratio() - start.ratio())),
        ),
        end_node,
        segment(runtime, false, None),
    ];
    if runtime.orientation == Orientation::Vertical {
        parts.reverse();
    }
    let root = control(
        container()
            .role(Role::Group)
            .label(name(label, None))
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
    let mut node = control(
        slider_semantics(runtime, name)
            .focusable(true)
            .hittable(false)
            .layout(LayoutStyle::block().shrink(0.0))
            .on_added(move |_, id| record.set(Some(id))),
        runtime.disabled,
    );
    let interaction = node.interaction();
    let node = node.child(slider_thumb(interaction, runtime.disabled, runtime.glass));
    let node = attach_key_handlers(node, interaction, runtime, Rc::clone(on_change));
    (node, Grip::focused(interaction, slot))
}

fn name(label: Option<&str>, part: Option<StringKey>) -> AccessibleName {
    let base = label.map(str::to_owned);
    match (base, part) {
        (Some(base), None) => AccessibleName::Static(base),
        (base, part) => {
            let fallback = theme_label(StringKey::Slider);
            let part = part.map(theme_label);
            AccessibleName::Derived(derived(move || {
                let base = base.clone().unwrap_or_else(|| fallback.get());
                part.map_or_else(|| base.clone(), |part| format!("{base} {}", part.get()))
            }))
        }
    }
}
