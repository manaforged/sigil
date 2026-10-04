use super::spinner;
use crate::kernel::{Cx, IntoNode, NodeId, NodeType};
use crate::reactive::Runtime;

fn has_text(cx: &Cx, id: NodeId) -> bool {
    let node = cx.get(id).expect("spinner node exists");
    matches!(node.kind, NodeType::Text { .. })
        || node.children().iter().any(|child| has_text(cx, *child))
}

#[cfg(feature = "scene")]
#[test]
fn window_spinner_draws_without_a_font_glyph() {
    Runtime::scope("spinner-window-glyph", || {
        let mut cx = Cx::new_root();
        let id = spinner().build(&mut cx);
        assert!(
            !has_text(&cx, id),
            "a window spinner must not depend on font coverage"
        );
    });
}
