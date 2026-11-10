#![allow(dead_code)]

use sigil::kernel::{Cx, NodeId};
use sigil::layout::{LayoutEngine, LayoutResult};
use sigil_kittest::FakeMeasure;

const TEXT_MEASURE: FakeMeasure = FakeMeasure::chars(8.0, 16.0);

#[allow(clippy::redundant_pub_crate)]
pub(crate) fn stub_engine() -> LayoutEngine {
    LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE))
}

pub(crate) fn layout_rect(cx: &Cx, id: NodeId) -> sigil::geometry::Rect {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("layout result exists")
        .rect()
}

pub(crate) fn first_child(cx: &Cx, root: NodeId) -> NodeId {
    cx.get(root)
        .and_then(|node| node.children().first().copied())
        .expect("root has a first child")
}
