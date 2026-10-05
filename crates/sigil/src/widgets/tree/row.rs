use crate::events::{MouseButton, NodeEventsExt};
use crate::kernel::{AccessibleName, NodeBuilder, Role, TextContent};
use crate::reactive::signal;

use super::super::patterns::{ListRowSlots, list_row_with};
use super::keys::{activate, select, toggle};
use super::{TreeKey, TreeRow, TreeState};

pub(super) fn render<K: TreeKey>(
    state: &TreeState<K>,
    index: usize,
    row: &TreeRow<K>,
) -> NodeBuilder {
    let node = &row.node;
    let slots = ListRowSlots {
        depth: row.depth,
        disclosure: row.disclosure,
        leading: node.leading.clone().map(|source| {
            Box::new(super::super::icon::icon(source)) as Box<dyn crate::kernel::IntoNode>
        }),
        status: node.status.as_ref().map(|(status, _)| status.clone()),
        status_tone: node.status.as_ref().map(|(_, tone)| *tone),
        detail: node.detail.clone(),
        tone: node.tone,
        indent: state.indent,
        ..ListRowSlots::default()
    };
    let level = usize::from(row.depth).saturating_add(1);
    let click_state = state.clone();
    let click_row = row.clone();
    list_row_with(row.selected, node.label.clone(), slots)
        .role(Role::TreeItem)
        .label(accessible_name(&node.label))
        .selected(signal(row.selected))
        .mutate(move |built| built.a11y.level = Some(level))
        .pointer_down(move |event| {
            if event
                .button
                .is_some_and(|button| button != MouseButton::Left)
            {
                return;
            }
            select(&click_state, index, &click_row);
            if click_row.has_children() {
                toggle(&click_state, &click_row);
            } else {
                activate(&click_state, &click_row);
            }
        })
}

pub(super) fn accessible_name(text: &TextContent) -> AccessibleName {
    match text {
        TextContent::Static(value) => AccessibleName::from(value.clone()),
        TextContent::Reactive(value) => AccessibleName::from(*value),
        TextContent::Compute(_) => AccessibleName::from(text.resolve()),
    }
}
