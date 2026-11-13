#![allow(clippy::unwrap_used)]

use sigil::events::EventRouter;
use sigil::kernel::{Cx, NodeId};
use sigil::layout::LayoutEngine;
use sigil::prelude::*;
use sigil::scene::{SceneCommand, emit_scene};

fn caret_rects(cx: &Cx, root: NodeId) -> Vec<sigil::geometry::Rect> {
    emit_scene(cx, root, None)
        .commands
        .iter()
        .filter_map(|c| match c {
            SceneCommand::Rect(r)
                if r.rect.width > 0.5
                    && r.rect.width < 3.0
                    && r.rect.height > 8.0
                    && r.rect.height < 40.0 =>
            {
                Some(r.rect)
            }
            _ => None,
        })
        .collect()
}

const OUTER_HEIGHT: f32 = 160.0;

#[test]
fn scrolling_keeps_the_end_of_buffer_caret_in_view() {
    sigil::reactive::Owner::scope("scroll-into-view", || {
        sigil::layout::text_geometry(std::rc::Rc::new(sigil::layout::TextEngineMeasure::default()));
        let buffer: String = (0..40)
            .map(|i| format!("line{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(signal(buffer)).multiline());
        let mut layout = LayoutEngine::new(Box::new(sigil::layout::TextEngineMeasure::default()));
        layout
            .compute(&mut cx, root, Constraint::fixed(400.0, 200.0))
            .expect("layout");
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        layout
            .compute(&mut cx, root, Constraint::fixed(400.0, 200.0))
            .expect("relayout after focus");

        let caret = caret_rects(&cx, root).first().copied().expect("caret");
        assert!(
            caret.origin.y >= 0.0 && caret.origin.y + caret.height <= OUTER_HEIGHT,
            "end-of-buffer caret must scroll into the visible box; got y={} h={} box={OUTER_HEIGHT}",
            caret.origin.y,
            caret.height,
        );
    });
}
