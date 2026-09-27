use super::text::RunNodes;
use crate::hal::Rect;
use crate::kernel::{A11yProps, AccessibleName, Cx, Node, NodeId, NodeType};
use accesskit::{
    Action, Node as AkNode, NodeId as AkNodeId, Toggled, TreeId, TreeInfo, TreeUpdate,
};

pub type BoundsLookup<'a> = &'a dyn Fn(NodeId) -> Option<Rect>;

#[must_use]
pub fn tree(cx: &Cx, focus: Option<NodeId>, bounds: BoundsLookup<'_>) -> TreeUpdate {
    let mut nodes = Vec::with_capacity(cx.len());
    let mut first_root_ak: Option<AkNodeId> = None;
    let roots = cx.roots();
    for root in roots.iter().copied() {
        walk_subtree(cx, root, bounds, &mut nodes, &mut first_root_ak);
    }
    let root_ak = first_root_ak.unwrap_or(AkNodeId(0));
    let focus_ak = focus
        .map(NodeId::to_accesskit)
        .filter(|candidate| nodes.iter().any(|(id, _)| id == candidate))
        .unwrap_or(root_ak);
    TreeUpdate {
        nodes,
        tree: Some(TreeInfo::new(root_ak)),
        tree_id: TreeId::ROOT,
        focus: focus_ak,
    }
}

fn walk_subtree(
    cx: &Cx,
    id: NodeId,
    bounds: BoundsLookup<'_>,
    out: &mut Vec<(AkNodeId, AkNode)>,
    first_root: &mut Option<AkNodeId>,
) {
    let Some(kernel_node) = cx.get(id) else {
        return;
    };
    if matches!(kernel_node.kind, NodeType::Fragment) {
        for &child in kernel_node.children() {
            walk_subtree(cx, child, bounds, out, first_root);
        }
        return;
    }
    let (projected, runs) = project(cx, id, bounds);
    let ak_id = id.to_accesskit();
    if first_root.is_none() {
        *first_root = Some(ak_id);
    }
    out.push((ak_id, projected));
    if let Some(runs) = runs {
        out.extend(runs);
        return;
    }
    for &child in kernel_node.children() {
        walk_subtree(cx, child, bounds, out, first_root);
    }
}

#[must_use]
pub fn node(cx: &Cx, id: NodeId, bounds: BoundsLookup<'_>) -> AkNode {
    project(cx, id, bounds).0
}

fn project(cx: &Cx, id: NodeId, bounds: BoundsLookup<'_>) -> (AkNode, Option<RunNodes>) {
    let Some(kernel_node) = cx.get(id) else {
        return (AkNode::new(accesskit::Role::Unknown), None);
    };
    let a11y = &kernel_node.a11y;
    let mut ak = AkNode::new(a11y.role);

    if a11y.labelled_by.is_none() {
        apply_name(&mut ak, &a11y.name);
    }
    apply_text_content(&mut ak, kernel_node);
    apply_description(&mut ak, a11y.description.as_ref());
    apply_labelled_by(&mut ak, a11y);
    apply_actions(&mut ak, a11y);
    apply_state(&mut ak, a11y);
    if !crate::events::hit_test::visible(cx, id) {
        ak.set_hidden();
    }
    apply_grid(&mut ak, a11y);
    apply_bounds(&mut ak, id, bounds, f64::from(cx.scale().factor()));
    let runs = super::text::export(id, a11y, &mut ak);
    if runs.is_none() {
        apply_children(cx, kernel_node, &mut ak);
    }
    (ak, runs)
}

fn apply_bounds(ak: &mut AkNode, id: NodeId, bounds: BoundsLookup<'_>, factor: f64) {
    let Some(rect) = bounds(id) else { return };
    ak.set_bounds(accesskit::Rect {
        x0: f64::from(rect.origin.x) * factor,
        y0: f64::from(rect.origin.y) * factor,
        x1: f64::from(rect.origin.x + rect.width) * factor,
        y1: f64::from(rect.origin.y + rect.height) * factor,
    });
}

fn apply_name(ak: &mut AkNode, name: &AccessibleName) {
    match name {
        AccessibleName::None => {}
        AccessibleName::Static(text) => ak.set_label(text.clone()),
        AccessibleName::Derived(signal) => ak.set_label(signal.peek()),
    }
}

fn apply_text_content(ak: &mut AkNode, node: &Node) {
    if !matches!(node.a11y.name, AccessibleName::None) || node.a11y.text_value.is_some() {
        return;
    }
    if let NodeType::Text { content, .. } = &node.kind {
        ak.set_value(content.resolve());
    }
}

fn apply_description(ak: &mut AkNode, description: Option<&AccessibleName>) {
    let Some(desc) = description else { return };
    match desc {
        AccessibleName::Static(text) => ak.set_description(text.clone()),
        AccessibleName::Derived(signal) => ak.set_description(signal.peek()),
        AccessibleName::None => {}
    }
}

fn apply_labelled_by(ak: &mut AkNode, a11y: &A11yProps) {
    if let Some(target) = a11y.labelled_by {
        ak.push_labelled_by(target.to_accesskit());
    }
}

fn apply_actions(ak: &mut AkNode, a11y: &A11yProps) {
    for action in &a11y.actions {
        ak.add_action(*action);
    }
    if let Some(shortcut) = &a11y.keyboard_shortcut {
        ak.set_keyboard_shortcut(format!("{shortcut:?}"));
    }
    ak.add_action(Action::ScrollIntoView);
    if a11y.is_disabled() {
        return;
    }
    if a11y.text_value.is_some() || a11y.value_now.is_some() {
        ak.add_action(Action::SetValue);
    }
    if let Some(sig) = a11y.expanded {
        ak.add_action(if sig.peek() {
            Action::Collapse
        } else {
            Action::Expand
        });
    }
}

fn apply_state(ak: &mut AkNode, a11y: &A11yProps) {
    if a11y.is_disabled() {
        ak.set_disabled();
    }
    if let Some(sig) = a11y.checked {
        let toggled = if sig.peek() {
            Toggled::True
        } else {
            Toggled::False
        };
        ak.set_toggled(toggled);
    }
    if let Some(sig) = a11y.selected {
        ak.set_selected(sig.peek());
    }
    if let (Some(now), Some(min), Some(max)) = (a11y.value_now, a11y.value_min, a11y.value_max) {
        ak.set_numeric_value(f64::from(now.peek()));
        ak.set_min_numeric_value(f64::from(min));
        ak.set_max_numeric_value(f64::from(max));
    }
    if let Some(step) = a11y.step {
        ak.set_numeric_value_step(f64::from(step));
    }
    if let Some(sig) = a11y.text_value {
        ak.set_value(sig.peek());
    }
    if let Some(placeholder) = &a11y.placeholder {
        ak.set_placeholder(placeholder.clone());
    }
    if let Some(sig) = a11y.expanded {
        ak.set_expanded(sig.peek());
    }
    if let Some(live) = a11y.live.or_else(|| default_live(a11y.role)) {
        ak.set_live(live);
    }
    if let Some(level) = a11y.level {
        ak.set_level(level);
    }
}

const fn default_live(role: accesskit::Role) -> Option<accesskit::Live> {
    match role {
        accesskit::Role::Alert => Some(accesskit::Live::Assertive),
        accesskit::Role::Status => Some(accesskit::Live::Polite),
        _ => None,
    }
}

fn apply_grid(ak: &mut AkNode, a11y: &A11yProps) {
    if let Some(sig) = a11y.row_count {
        ak.set_row_count(sig.peek());
    }
    if let Some(count) = a11y.column_count {
        ak.set_column_count(count);
    }
    if let Some(index) = a11y.row_index {
        ak.set_row_index(index);
    }
    if let Some(index) = a11y.column_index {
        ak.set_column_index(index);
    }
    if let Some(sig) = a11y.sort_direction
        && let Some(direction) = sig.peek()
    {
        ak.set_sort_direction(direction);
    }
}

fn apply_children(cx: &Cx, node: &Node, ak: &mut AkNode) {
    let mut effective = Vec::with_capacity(node.children().len());
    for &child in node.children() {
        collect_effective(cx, child, &mut effective);
    }
    for child_id in effective {
        ak.push_child(child_id.to_accesskit());
    }
}

fn collect_effective(cx: &Cx, id: NodeId, out: &mut Vec<NodeId>) {
    let Some(node) = cx.get(id) else { return };
    if matches!(node.kind, NodeType::Fragment) {
        for &grandchild in node.children() {
            collect_effective(cx, grandchild, out);
        }
    } else {
        out.push(id);
    }
}
