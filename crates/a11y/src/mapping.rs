//! Kernel-node → `accesskit::Node` projection (DR-050 §2).
//!
//! `node` reads one [`sigil_kernel::Node`]'s [`A11yProps`] and
//! returns the AccessKit projection. `tree` walks the kernel
//! arena depth-first from every root, skipping fragments, and produces
//! the full `TreeUpdate`.

use accesskit::{Node as AkNode, NodeId as AkNodeId, Toggled, Tree, TreeId, TreeUpdate};
use sigil_kernel::{A11yProps, AccessibleName, Cx, Node, NodeId, NodeType};

/// Build the AccessKit tree for the current [`Cx`] state.
///
/// One entry per non-fragment node, in depth-first document order. The
/// `focus` field defaults to the first root when no explicit focus is
/// provided — AccessKit requires a non-zero focus node and uses the
/// root as the sentinel.
#[must_use]
pub fn tree(cx: &Cx, focus: Option<NodeId>) -> TreeUpdate {
    let mut nodes = Vec::with_capacity(cx.len());
    let mut first_root_ak: Option<AkNodeId> = None;
    let roots = cx.roots();
    for root in roots.iter().copied() {
        walk_subtree(cx, root, &mut nodes, &mut first_root_ak);
    }
    let root_ak = first_root_ak.unwrap_or(AkNodeId(0));
    let focus_ak = focus.map_or(root_ak, NodeId::to_accesskit);
    TreeUpdate {
        nodes,
        tree: Some(Tree::new(root_ak)),
        tree_id: TreeId::ROOT,
        focus: focus_ak,
    }
}

fn walk_subtree(
    cx: &Cx,
    id: NodeId,
    out: &mut Vec<(AkNodeId, AkNode)>,
    first_root: &mut Option<AkNodeId>,
) {
    let Some(kernel_node) = cx.get(id) else {
        return;
    };
    if matches!(kernel_node.kind, NodeType::Fragment) {
        for &child in kernel_node.children() {
            walk_subtree(cx, child, out, first_root);
        }
        return;
    }
    let projected = node(cx, id);
    let ak_id = id.to_accesskit();
    if first_root.is_none() {
        *first_root = Some(ak_id);
    }
    out.push((ak_id, projected));
    for &child in kernel_node.children() {
        walk_subtree(cx, child, out, first_root);
    }
}

/// Project one kernel node into an [`AkNode`].
///
/// Signal reads use [`sigil_reactive::Signal::peek`] (untracked) — the
/// observer registered on attach is the subscription path; projecting
/// must not register duplicate subscriptions every frame.
#[must_use]
pub fn node(cx: &Cx, id: NodeId) -> AkNode {
    let Some(kernel_node) = cx.get(id) else {
        return AkNode::new(accesskit::Role::Unknown);
    };
    let a11y = &kernel_node.a11y;
    let mut ak = AkNode::new(a11y.role);

    apply_name(&mut ak, &a11y.name);
    apply_description(&mut ak, a11y.description.as_ref());
    apply_labelled_by(&mut ak, a11y);
    apply_actions(&mut ak, a11y);
    apply_state(&mut ak, a11y);
    apply_children(cx, kernel_node, &mut ak);
    ak
}

fn apply_name(ak: &mut AkNode, name: &AccessibleName) {
    match name {
        AccessibleName::None => {}
        AccessibleName::Static(text) => ak.set_label(text.clone()),
        AccessibleName::Derived(signal) => ak.set_label(signal.peek()),
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
    if !a11y.hittable {
        ak.set_hidden();
    }
}

fn apply_state(ak: &mut AkNode, a11y: &A11yProps) {
    if let Some(sig) = a11y.checked {
        let toggled = if sig.peek() {
            Toggled::True
        } else {
            Toggled::False
        };
        ak.set_toggled(toggled);
    }
    if let (Some(now), Some(min), Some(max)) = (a11y.value_now, a11y.value_min, a11y.value_max) {
        ak.set_numeric_value(f64::from(now.peek()));
        ak.set_min_numeric_value(f64::from(min));
        ak.set_max_numeric_value(f64::from(max));
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
    if let Some(live) = a11y.live {
        ak.set_live(live);
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
