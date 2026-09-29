#![allow(dead_code)]

use sigil::geometry::Size;
use sigil::kernel::{Cx, NodeId, NodeType};
use sigil::layout::{LayoutEngine, TextMeasure};
use sigil::scene::{SceneCommand, emit_scene};
use sigil::style::Rgba8;

pub(crate) fn compute(
    cx: &mut Cx,
    root: NodeId,
    viewport: Size,
    measure: Box<dyn TextMeasure>,
) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(measure);
    layout.compute(cx, root, viewport).expect("layout computes");
    layout
}

pub(crate) fn rect_colors(cx: &Cx, root: NodeId) -> Vec<Rgba8> {
    emit_scene(cx, root)
        .commands
        .iter()
        .filter_map(|c| match c {
            SceneCommand::Rect(r) => Some(r.color),
            _ => None,
        })
        .collect()
}

pub(crate) fn find_text_node(cx: &Cx, root: NodeId) -> Option<NodeId> {
    let mut found = None;
    cx.visit_preorder(root, |id, node| {
        if found.is_none() && matches!(node.kind, NodeType::Text { .. }) {
            found = Some(id);
        }
    });
    found
}

pub(crate) fn assert_approx(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.001,
        "expected {expected}, got {actual}"
    );
}
