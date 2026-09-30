#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::float_cmp,
    clippy::shadow_unrelated
)]

use sigil::kernel::AccessibleName;
use sigil::kernel::{Cx, NodeId};
use sigil::layout::LayoutEngine;
use sigil::prelude::*;
use sigil_kittest::stub_text_measure;

const VIEWPORT: Size = Size::fixed(640.0, 480.0);

fn fresh_engine() -> LayoutEngine {
    LayoutEngine::with_text_measure(stub_text_measure())
}

fn recompute(layout: &mut LayoutEngine, cx: &mut Cx, root: NodeId) {
    layout.compute(cx, root, VIEWPORT).expect("layout computes");
}

fn find_by_role_label(cx: &Cx, root: NodeId, role: Role, label: &str) -> Vec<NodeId> {
    let mut hits = Vec::new();
    cx.visit_preorder(root, |id, node| {
        if node.a11y.role == role && matches_static_label(&node.a11y.name, label) {
            hits.push(id);
        }
    });
    hits
}

fn matches_static_label(name: &AccessibleName, expected: &str) -> bool {
    matches!(name, AccessibleName::Static(value) if value == expected)
}

#[test]
fn every_tab_stays_mounted_and_only_the_active_one_shows() {
    sigil::reactive::Runtime::scope("sigil-tabs-kept-panels", || {
        let active = signal(0_usize);
        let mut cx = Cx::new_root();
        let root = cx.add(
            tabs()
                .active(active)
                .tab("A", || {
                    container()
                        .label("panel-0")
                        .layout(LayoutStyle::block().fixed_size(120.0, 40.0))
                })
                .tab("B", || {
                    container()
                        .label("panel-1")
                        .layout(LayoutStyle::block().fixed_size(120.0, 40.0))
                }),
        );

        let mut layout = fresh_engine();
        recompute(&mut layout, &mut cx, root);
        let panel0 = find_by_role_label(&cx, root, Role::GenericContainer, "panel-0");
        let panel1 = find_by_role_label(&cx, root, Role::GenericContainer, "panel-1");
        assert_eq!(panel0.len(), 1, "panel-0 is in the tree");
        assert_eq!(panel1.len(), 1, "panel-1 is in the tree");

        let h0 = layout.result(panel0[0]).map_or(0.0, |r| r.rect().height);
        let h1 = layout.result(panel1[0]).map_or(0.0, |r| r.rect().height);
        assert!(h0 > 0.0, "active=0 shows panel-0 (height = {h0})");
        assert_eq!(h1, 0.0, "active=0 hides panel-1 (height = {h1})");

        active.set(1);
        recompute(&mut layout, &mut cx, root);
        assert_eq!(
            find_by_role_label(&cx, root, Role::GenericContainer, "panel-0"),
            panel0,
            "switching keeps the same panel-0 node"
        );
        let h0 = layout.result(panel0[0]).map_or(0.0, |r| r.rect().height);
        let h1 = layout.result(panel1[0]).map_or(0.0, |r| r.rect().height);
        assert_eq!(h0, 0.0, "active=1 hides panel-0 (height = {h0})");
        assert!(h1 > 0.0, "active=1 shows panel-1 (height = {h1})");
    });
}
