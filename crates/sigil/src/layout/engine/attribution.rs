use crate::kernel::{Cx, NodeId};
use crate::profile::{Counter, Hotspot};

pub(super) fn note_dirty(cx: &Cx, dirty: &[NodeId], kernel_len: usize) {
    if !crate::profile::enabled() {
        return;
    }
    let split = kernel_len.min(dirty.len());
    let (kernel, seeded) = dirty.split_at(split);
    crate::profile::count_add(Counter::LayoutDirtyKernel, kernel.len() as u64);
    crate::profile::count_add(Counter::LayoutDirtySeed, seeded.len() as u64);
    for id in kernel.iter().copied() {
        crate::profile::attribute(Hotspot::LayoutDirtyKernel, 1, || cx.describe(id));
    }
    for id in seeded.iter().copied() {
        crate::profile::attribute(Hotspot::LayoutDirtySeed, 1, || cx.describe(id));
    }
}

pub(super) fn note_resized(cx: &Cx, resized: &[NodeId]) {
    if !crate::profile::enabled() {
        return;
    }
    for id in resized.iter().copied() {
        crate::profile::attribute(Hotspot::LayoutResized, 1, || cx.describe(id));
    }
}

pub(super) fn note_roots(cx: &Cx, roots: &[NodeId], resized: &[NodeId]) {
    if !crate::profile::enabled() {
        return;
    }
    for root in roots.iter().copied() {
        let nodes = subtree_len(cx, root);
        crate::profile::count_add(Counter::LayoutSolvedNodes, nodes);
        let trigger = resized
            .iter()
            .copied()
            .find(|id| contains(cx, root, *id))
            .or_else(|| resized.first().copied());
        crate::profile::attribute(Hotspot::LayoutRoot, nodes, || {
            trigger.map_or_else(
                || cx.describe(root),
                |trigger| format!("{} <- {}", cx.describe(root), cx.describe(trigger)),
            )
        });
    }
}

pub(super) fn note_full_root(cx: &Cx, root: NodeId, reason: Counter) {
    if !crate::profile::enabled() {
        return;
    }
    let nodes = subtree_len(cx, root);
    crate::profile::count_add(Counter::LayoutSolvedNodes, nodes);
    crate::profile::attribute(Hotspot::LayoutRoot, nodes, || format!("full: {reason}"));
}

fn subtree_len(cx: &Cx, root: NodeId) -> u64 {
    let mut nodes = 0_u64;
    cx.visit_preorder(root, |_, _| nodes += 1);
    nodes
}

pub(super) fn contains(cx: &Cx, ancestor: NodeId, id: NodeId) -> bool {
    let mut current = Some(id);
    while let Some(node) = current {
        if node == ancestor {
            return true;
        }
        current = cx.parent(node);
    }
    false
}
