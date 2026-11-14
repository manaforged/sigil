use crate::events::MouseButton;
use crate::kernel::{Action, NodeBuilder, Role};
use crate::reactive::signal;
use crate::widgets::Widget;

use super::super::patterns::list_row;
use super::super::shared::accessible_name;
use super::keys::{activate, select, toggle};
use core::hash::Hash;

use super::{TreeRow, TreeState};

pub(super) fn render<K: Clone + Eq + Hash + 'static>(
    state: &TreeState<K>,
    index: usize,
    row: &TreeRow<K>,
) -> NodeBuilder {
    let node = &row.node;
    let mut item = list_row(node.label.clone())
        .selected(row.selected)
        .depth(row.depth)
        .disclosure(row.disclosure)
        .tone(node.tone);
    if let Some(indent) = state.indent {
        item = item.indent(indent);
    }
    if let Some(source) = node.leading.clone() {
        item = item.leading(super::super::icon::icon(source));
    }
    if let Some((status, tone)) = node.status.clone() {
        item = item.status(status).status_tone(tone);
    }
    if let Some(detail) = node.detail.clone() {
        item = item.detail(detail);
    }
    let level = usize::from(row.depth).saturating_add(1);
    let click_state = state.clone();
    let click_row = row.clone();
    item.into_node_builder()
        .role(Role::TreeItem)
        .accessible(accessible_name(&node.label))
        .selected(signal(row.selected))
        .mutate(move |built| built.a11y.level = Some(level))
        .action(Action::Click)
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
