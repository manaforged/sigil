#![allow(missing_docs)]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::prelude::*;
use sigil_renderer::{SceneCommand, emit_scene};

fn caret_rects(cx: &Cx, root: NodeId) -> Vec<sigil_hal::Rect> {
    emit_scene(cx, root)
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
    sigil::reactive::Runtime::scope("scroll-into-view", || {
        sigil_layout::set_text_geometry(std::rc::Rc::new(sigil_app::EngineGeometry::new()));
        let buffer: String = (0..40)
            .map(|i| format!("line{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut cx = Cx::new_root();
        let root = cx.add(text_edit(signal(buffer)));
        let mut layout =
            LayoutEngine::with_text_measure(Box::new(sigil_app::DefaultTextMeasure::new()));
        layout
            .compute(&mut cx, root, Size::fixed(400.0, 200.0))
            .expect("layout");
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        layout
            .compute(&mut cx, root, Size::fixed(400.0, 200.0))
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
