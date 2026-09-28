use std::collections::HashMap;
use std::fmt::Write as _;

use accesskit::{Node as AkNode, NodeId as AkNodeId, TreeUpdate};

#[must_use]
pub fn snapshot(update: &TreeUpdate) -> String {
    let lookup: HashMap<AkNodeId, &AkNode> =
        update.nodes.iter().map(|(id, node)| (*id, node)).collect();
    let mut out = String::new();
    let root = update.tree.as_ref().map_or(AkNodeId(0), |tree| tree.root);
    let focus = update.focus;
    match lookup.get(&root) {
        Some(window) if root == super::mapping::WINDOW_ROOT => {
            for child in window.children() {
                walk(&lookup, *child, focus, 0, &mut out);
            }
        }
        _ => walk(&lookup, root, focus, 0, &mut out),
    }
    out
}

fn walk(
    lookup: &HashMap<AkNodeId, &AkNode>,
    id: AkNodeId,
    focus: AkNodeId,
    depth: usize,
    out: &mut String,
) {
    let Some(node) = lookup.get(&id) else {
        return;
    };
    for _ in 0..depth {
        out.push_str("  ");
    }
    write_node_line(node, id == focus, out);
    out.push('\n');
    for child in node.children() {
        walk(lookup, *child, focus, depth + 1, out);
    }
}

fn write_node_line(node: &AkNode, focused: bool, out: &mut String) {
    let _ = write!(out, "[Role={:?}", node.role());
    if let Some(label) = node.label() {
        let _ = write!(out, ", Name={label:?}");
    }
    if let Some(description) = node.description() {
        let _ = write!(out, ", Description={description:?}");
    }
    if let Some(level) = node.level() {
        let _ = write!(out, ", Level={level}");
    }
    if let Some(selected) = node.is_selected() {
        let _ = write!(out, ", Selected={selected}");
    }
    if let Some(value) = node.value() {
        let _ = write!(out, ", Value={value:?}");
    }
    if let Some(placeholder) = node.placeholder() {
        let _ = write!(out, ", Placeholder={placeholder:?}");
    }
    if let Some(toggled) = node.toggled() {
        let _ = write!(out, ", Toggled={toggled:?}");
    }
    if let Some(numeric) = node.numeric_value() {
        let _ = write!(out, ", Numeric={numeric}");
    }
    if let Some(expanded) = node.is_expanded() {
        let _ = write!(out, ", Expanded={expanded}");
    }
    if let Some(live) = node.live() {
        let _ = write!(out, ", Live={live:?}");
    }
    if focused {
        out.push_str(", Focused=true");
    }
    out.push(']');
}
