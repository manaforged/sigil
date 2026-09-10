#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use accesskit::{Node as AkNode, Role, TreeUpdate};
use sigil::widgets::{button, modal, text_field};
use sigil_a11y::tree;
use sigil_hal::{Point, Rect};
use sigil_kernel::{Cx, NodeId, TextContent, container};
use sigil_layout::prelude::*;
use sigil_reactive::{Runtime, signal};

struct ZeroTextMeasure;

impl TextMeasure for ZeroTextMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::default()
    }
}

#[test]
fn bounds_reflect_a_signal_driven_move_only_after_a_fresh_recompute() {
    Runtime::scope("sigil_a11y::bounds_freshness", || {
        let mut cx = Cx::new_root();
        let pos = signal((10.0_f32, 20.0_f32));
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(400.0, 400.0))
                .child(
                    container()
                        .role(Role::Button)
                        .label("Card")
                        .layout(LayoutStyle::block().fixed_size(80.0, 32.0))
                        .position_reactive(pos),
                ),
        );
        let moved = *cx
            .get(root)
            .expect("root must exist")
            .children()
            .first()
            .expect("root has one child");
        let mut layout = LayoutEngine::with_text_measure(Box::new(ZeroTextMeasure));
        recompute(&mut cx, &mut layout, root);
        let before = projected_bounds(&cx, &layout, moved);
        assert_eq!(before, (10.0, 20.0), "starts at the signal's initial pos");

        pos.set((200.0, 250.0));
        recompute(&mut cx, &mut layout, root);
        let after = projected_bounds(&cx, &layout, moved);

        assert_eq!(
            after,
            (200.0, 250.0),
            "reflects the moved pos after recompute"
        );
        assert_ne!(
            before, after,
            "the SAME tree() call, given a bounds closure over a freshly \
             recomputed layout, must carry the new bounds, not the stale ones"
        );
    });
}

fn recompute(cx: &mut Cx, layout: &mut LayoutEngine, root: NodeId) {
    layout
        .compute(cx, root, Size::fixed(400.0, 400.0))
        .expect("layout compute must succeed");
}

fn projected_bounds(cx: &Cx, layout: &LayoutEngine, id: NodeId) -> (f64, f64) {
    let update = tree(cx, None, &|node_id| {
        layout.result(node_id).map(|result| result.rect())
    });
    let bounds = find(&update, id)
        .expect("node must project")
        .bounds()
        .expect("node must have bounds after layout");
    (bounds.x0, bounds.y0)
}

#[test]
fn projects_role_name_and_injected_bounds_for_button_field_and_modal() {
    Runtime::scope("sigil_a11y::bounds_projection", || {
        let mut cx = Cx::new_root();

        let button_id = build_button(&mut cx);
        let field_id = build_text_field(&mut cx);
        let modal_id = build_modal(&mut cx);

        let button_rect = Rect::new(Point::new(10.0, 20.0), 80.0, 32.0);
        let field_rect = Rect::new(Point::new(10.0, 60.0), 200.0, 28.0);
        let modal_rect = Rect::new(Point::new(0.0, 0.0), 400.0, 300.0);
        let bounds = move |id: NodeId| -> Option<Rect> {
            if id == button_id {
                Some(button_rect)
            } else if id == field_id {
                Some(field_rect)
            } else if id == modal_id {
                Some(modal_rect)
            } else {
                None
            }
        };

        let update = tree(&cx, Some(button_id), &bounds);

        let button_node = find(&update, button_id).expect("button must project a node");
        assert_eq!(button_node.role(), Role::Button, "button role");
        assert_eq!(button_node.label(), Some("Save"), "button label");
        assert_rect_eq(
            button_node.bounds().expect("button bounds must be set"),
            button_rect,
            "button bounds",
        );

        let field_node = find(&update, field_id).expect("text field must project a node");
        assert_eq!(field_node.role(), Role::TextInput, "text field role");
        assert_eq!(field_node.label(), Some("Email"), "text field label");
        assert_rect_eq(
            field_node.bounds().expect("field bounds must be set"),
            field_rect,
            "text field bounds",
        );

        let modal_root = find(&update, modal_id).expect("modal root must project a node");
        assert_rect_eq(
            modal_root.bounds().expect("modal root bounds must be set"),
            modal_rect,
            "modal root bounds",
        );
        let dialog_node =
            find_by_role(&update, Role::Dialog).expect("modal must contain a Role::Dialog node");
        assert_eq!(dialog_node.label(), Some("Settings"), "modal dialog label");
    });
}

fn build_button(cx: &mut Cx) -> NodeId {
    cx.add(button("Save"))
}

fn build_text_field(cx: &mut Cx) -> NodeId {
    let value = signal(String::new());
    cx.add(text_field(value).placeholder("Email"))
}

fn build_modal(cx: &mut Cx) -> NodeId {
    let show = signal(true);
    cx.add(modal(show).title("Settings"))
}

fn find(update: &TreeUpdate, id: NodeId) -> Option<&AkNode> {
    let target = id.to_accesskit();
    update
        .nodes
        .iter()
        .find_map(|(node_id, node)| (*node_id == target).then_some(node))
}

fn find_by_role(update: &TreeUpdate, role: Role) -> Option<&AkNode> {
    update
        .nodes
        .iter()
        .map(|(_, node)| node)
        .find(|node| node.role() == role)
}

fn assert_rect_eq(actual: accesskit::Rect, expected: Rect, what: &str) {
    let expected_ak = accesskit::Rect {
        x0: f64::from(expected.origin.x),
        y0: f64::from(expected.origin.y),
        x1: f64::from(expected.origin.x + expected.width),
        y1: f64::from(expected.origin.y + expected.height),
    };
    assert_eq!(actual, expected_ak, "{what} did not match injected rect");
}
