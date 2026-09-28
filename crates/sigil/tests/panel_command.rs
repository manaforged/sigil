#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

#[path = "widget_support/mod.rs"]
mod widget_support;

use sigil::prelude::*;
use sigil::scene::emit_scene;
use sigil_kittest::stub_text_measure;
use widget_support::rect_colors;

#[test]
fn panel_has_generic_container_role() {
    sigil::reactive::Runtime::scope("sigil-panel-role", || {
        let mut cx = Cx::new_root();
        let root = cx.add(panel());
        let node = cx.get(root).unwrap();
        assert_eq!(node.a11y.role, Role::GenericContainer);
    });
}

#[test]
fn panel_elevation_zero_emits_surface_color() {
    sigil::reactive::Runtime::scope("sigil-panel-elevation-0", || {
        let mut cx = Cx::new_root();
        let root = cx.add(panel().elevation(0));
        let mut layout = LayoutEngine::with_text_measure(stub_text_measure());
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 480.0))
            .expect("layout computes");

        let theme = Theme::default();
        let colors = rect_colors(&cx, root);
        assert!(
            colors.contains(&theme.surface),
            "elevation 0 should emit surface color"
        );
    });
}

#[test]
fn panel_elevation_one_emits_surface_hover_color() {
    sigil::reactive::Runtime::scope("sigil-panel-elevation-1", || {
        let mut cx = Cx::new_root();
        let root = cx.add(panel().elevation(1));
        let mut layout = LayoutEngine::with_text_measure(stub_text_measure());
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 480.0))
            .expect("layout computes");

        let theme = Theme::default();
        let colors = rect_colors(&cx, root);
        assert!(
            colors.contains(&theme.surface_hover),
            "elevation 1 should emit surface-hover color"
        );
    });
}

#[test]
fn panel_with_title_renders_text() {
    sigil::reactive::Runtime::scope("sigil-panel-title", || {
        let mut cx = Cx::new_root();
        let root = cx.add(panel().title("Inventory"));
        let mut layout = LayoutEngine::with_text_measure(stub_text_measure());
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 480.0))
            .expect("layout computes");

        assert!(
            scene_has_text(&cx, root, "Inventory"),
            "panel title should appear in scene"
        );
    });
}

#[test]
fn panel_without_title_emits_no_title_text() {
    sigil::reactive::Runtime::scope("sigil-panel-no-title", || {
        let mut cx = Cx::new_root();
        let root = cx.add(panel());
        let mut layout = LayoutEngine::with_text_measure(stub_text_measure());
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 480.0))
            .expect("layout computes");

        assert!(
            !scene_has_text(&cx, root, "Inventory"),
            "untitled panel should not emit title text"
        );
    });
}

#[test]
fn panel_with_child_label_renders_child_text() {
    sigil::reactive::Runtime::scope("sigil-panel-child", || {
        let mut cx = Cx::new_root();
        let root = cx.add(panel().title("Stats").child(text("Gold: 42").nowrap()));
        let mut layout = LayoutEngine::with_text_measure(stub_text_measure());
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 480.0))
            .expect("layout computes");

        assert!(scene_has_text(&cx, root, "Stats"), "title in scene");
        assert!(scene_has_text(&cx, root, "Gold: 42"), "child text in scene");
    });
}

fn scene_has_text(cx: &Cx, root: NodeId, expected: &str) -> bool {
    emit_scene(cx, root).find_text(expected).is_some()
}
