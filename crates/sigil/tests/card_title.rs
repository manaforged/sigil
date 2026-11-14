#![allow(clippy::unwrap_used)]

use sigil::geometry::Constraint;
use sigil::kernel::{Cx, NodeId};
use sigil::layout::LayoutEngine;
use sigil::prelude::*;
use sigil::scene::emit_scene;
use sigil_kittest::stub_text_measure;

#[test]
fn card_has_generic_container_role() {
    sigil::reactive::Owner::scope("sigil-card-role", || {
        let mut cx = Cx::new_root();
        let root = cx.add(card(text("Body")));
        let node = cx.get(root).unwrap();
        assert_eq!(node.a11y.role, Role::GenericContainer);
    });
}

#[test]
fn card_with_title_renders_text() {
    sigil::reactive::Owner::scope("sigil-card-title", || {
        let mut cx = Cx::new_root();
        let root = cx.add(card(text("Body")).title("Inventory"));
        let mut layout = LayoutEngine::new(stub_text_measure());
        layout
            .compute(&mut cx, root, Constraint::fixed(640.0, 480.0))
            .expect("layout computes");

        assert!(
            scene_has_text(&cx, root, "Inventory"),
            "card title should appear in scene"
        );
    });
}

#[test]
fn card_without_title_emits_no_title_text() {
    sigil::reactive::Owner::scope("sigil-card-no-title", || {
        let mut cx = Cx::new_root();
        let root = cx.add(card(text("Body")));
        let mut layout = LayoutEngine::new(stub_text_measure());
        layout
            .compute(&mut cx, root, Constraint::fixed(640.0, 480.0))
            .expect("layout computes");

        assert!(
            !scene_has_text(&cx, root, "Inventory"),
            "untitled card should not emit title text"
        );
    });
}

#[test]
fn card_with_child_label_renders_child_text() {
    sigil::reactive::Owner::scope("sigil-card-child", || {
        let mut cx = Cx::new_root();
        let root = cx.add(card(text("Gold: 42").nowrap()).title("Stats"));
        let mut layout = LayoutEngine::new(stub_text_measure());
        layout
            .compute(&mut cx, root, Constraint::fixed(640.0, 480.0))
            .expect("layout computes");

        assert!(scene_has_text(&cx, root, "Stats"), "title in scene");
        assert!(scene_has_text(&cx, root, "Gold: 42"), "child text in scene");
    });
}

fn scene_has_text(cx: &Cx, root: NodeId, expected: &str) -> bool {
    emit_scene(cx, root, None).find_text(expected).is_some()
}
