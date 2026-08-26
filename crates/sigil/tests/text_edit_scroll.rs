//! `text_edit` scrolls the content so the caret stays in view.
//!
//! Before this, a document taller than the fixed edit box painted every
//! line at a fixed offset: the caret on a line past the viewport rendered
//! outside the box (or was clipped away). The editor now carries a vertical
//! scroll offset driven by an effect that keeps the caret line inside the
//! visible band, so an end-of-buffer caret in a tall document paints inside
//! the box.

#![allow(missing_docs)]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::prelude::*;
use sigil_renderer::{SceneCommand, emit_scene};

/// The caret is a thin (~1.5px), short (one line-height) rect.
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

/// Outer box height (`DEFAULT_HEIGHT` in the widget). The caret must paint
/// inside this in a tall document, not below it.
const OUTER_HEIGHT: f32 = 160.0;

#[test]
fn scrolling_keeps_the_end_of_buffer_caret_in_view() {
    sigil::reactive::Runtime::scope("scroll-into-view", || {
        sigil_layout::set_text_geometry(std::rc::Rc::new(sigil_app::EngineGeometry::new()));
        // 40 lines — far more than the ~9 that fit in the fixed box, so the
        // end-of-buffer caret starts well past the viewport.
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
        // Line 39 at a ~16px line height sits ~624px down the document —
        // without scroll the caret would paint far below the 160px box.
        assert!(
            caret.origin.y >= 0.0 && caret.origin.y + caret.height <= OUTER_HEIGHT,
            "end-of-buffer caret must scroll into the visible box; got y={} h={} box={OUTER_HEIGHT}",
            caret.origin.y,
            caret.height,
        );
    });
}
