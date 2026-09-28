use accesskit::{Action, ActionData, ActionRequest};

use crate::events::{A11yEdit, EventRouter, dispatch};
use crate::kernel::{Cx, NodeId};
use crate::layout::LayoutEngine;

pub(crate) fn apply(
    cx: &mut Cx,
    router: &mut EventRouter,
    layout: &LayoutEngine,
    target: NodeId,
    request: &ActionRequest,
) {
    router.focus_if_needed(cx, layout);
    let focusable = router.is_focus_target(cx, layout, target);
    let clickable = router.is_click_target(cx, layout, target);
    match request.action {
        Action::Focus if focusable => router.set_focus(cx, Some(target)),
        Action::Click if clickable => {
            crate::events::pointer_dispatch::invoke_node(cx, layout, target);
        }
        Action::Increment if focusable => adjust_value(cx, target, true),
        Action::Decrement if focusable => adjust_value(cx, target, false),
        Action::SetValue | Action::SetTextSelection if focusable => {
            edit(cx, target, request.data.as_ref());
        }
        Action::Expand | Action::Collapse if clickable => {
            set_expanded(cx, target, request.action == Action::Expand);
        }
        Action::ScrollIntoView => crate::events::focus::reveal_focus(cx, layout, target),
        _ => {}
    }
}

fn adjust_value(cx: &Cx, target: NodeId, increase: bool) {
    if dispatch(cx, target, |handlers| {
        handlers.invoke_value_adjustment(increase)
    }) == Some(true)
    {
        return;
    }
    let Some(node) = cx.get(target) else {
        return;
    };
    let (Some(value), Some(min), Some(max)) = (
        node.a11y.value_now,
        node.a11y.value_min,
        node.a11y.value_max,
    ) else {
        return;
    };
    let Some(current) = value.try_peek() else {
        return;
    };
    let step = node.a11y.step.unwrap_or(1.0);
    let next = if increase {
        current + step
    } else {
        current - step
    };
    value.try_set(next.max(min).min(max));
}

fn set_expanded(cx: &Cx, target: NodeId, expand: bool) {
    let Some(node) = cx.get(target) else { return };
    if node.a11y.is_disabled() {
        return;
    }
    let Some(signal) = node.a11y.expanded else {
        return;
    };
    if signal.peek() != expand {
        signal.set(expand);
    }
}

fn edit(cx: &Cx, target: NodeId, data: Option<&ActionData>) {
    let Some(edit) = edit_request(cx, target, data) else {
        return;
    };
    if dispatch(cx, target, |handlers| handlers.invoke_a11y_edit(&edit)) == Some(true) {
        return;
    }
    let Some(node) = cx.get(target) else { return };
    let a11y = &node.a11y;
    match edit {
        A11yEdit::Number(value) => {
            if let (Some(signal), Some(min), Some(max)) =
                (a11y.value_now, a11y.value_min, a11y.value_max)
            {
                signal.set(value.clamp(min, max));
            }
        }
        A11yEdit::Text(text) => {
            if let Some(signal) = a11y.text_value {
                signal.set(text);
            }
        }
        A11yEdit::Select { .. } => {}
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "accessibility numeric values arrive as f64; widget values are f32"
)]
fn edit_request(cx: &Cx, target: NodeId, data: Option<&ActionData>) -> Option<A11yEdit> {
    let node = cx.get(target)?;
    if node.a11y.is_disabled() {
        return None;
    }
    let edit = match data? {
        ActionData::NumericValue(value) => A11yEdit::Number(*value as f32),
        ActionData::Value(text) if node.a11y.value_now.is_some() => {
            A11yEdit::Number(text.trim().parse::<f32>().ok()?)
        }
        ActionData::Value(text) => A11yEdit::Text(text.to_string()),
        ActionData::SetTextSelection(selection) => {
            let (anchor, focus) = super::text::selection_bytes(target, &node.a11y, *selection)?;
            A11yEdit::Select { anchor, focus }
        }
        _ => return None,
    };
    match edit {
        A11yEdit::Number(value) if !value.is_finite() => None,
        edit => Some(edit),
    }
}
