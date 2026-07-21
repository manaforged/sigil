//! `text_edit` paints a visible caret that tracks the buffer offset across
//! lines.
//!
//! Before this, `text_edit` had no caret overlay at all (the runtime comment
//! said so). The caret geometry is multi-line: the byte offset maps to a
//! (line, column), the column gives x via the text engine and the line
//! gives y. Verified against the "not a production editor" review.

#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
#![allow(clippy::unwrap_used)]

use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_style as _;
use sigil_tasks as _;
use wgpu as _;

use sigil::prelude::*;
use sigil_renderer::{SceneCommand, emit_scene};

/// The caret is a thin (~1.5px), short (one line-height) rect. Border edges
/// are also thin but full-height, so both bounds are needed to isolate it.
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

fn focused_edit(buffer: &str) -> (Cx, NodeId, LayoutEngine) {
    // The caret reads the installed text engine's geometry; install it as
    // the app does (the golden harness does not).
    sigil_layout::set_text_geometry(std::rc::Rc::new(sigil_app::EngineGeometry::new()));

    let mut cx = Cx::new_root();
    let root = cx.add(text_edit(signal(buffer.to_owned())));
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
    (cx, root, layout)
}

#[test]
fn caret_is_painted_when_focused() {
    sigil::reactive::Runtime::scope("caret-present", || {
        let (cx, root, _layout) = focused_edit("hello");
        assert!(
            !caret_rects(&cx, root).is_empty(),
            "a focused text_edit must paint a caret",
        );
    });
}

#[test]
fn caret_sits_on_the_line_its_offset_is_on() {
    sigil::reactive::Runtime::scope("caret-line", || {
        // Caret defaults to the end of the buffer — here, line index 2.
        let (cx, root, _layout) = focused_edit("aaa\nbbb\nccc");
        let carets = caret_rects(&cx, root);
        let caret = carets.first().expect("caret present");
        // A single-line caret would sit near the top padding (~8px). On the
        // third line it must be at least two line-heights below that.
        assert!(
            caret.origin.y > 8.0 + caret.height,
            "caret for an end-of-buffer offset on line 3 should be well below \
             the first line; got y={} h={}",
            caret.origin.y,
            caret.height,
        );
    });
}

use sigil_hal::{MouseButton, MouseButtons, Point, PointerEventInit};

fn click_at(cx: &mut Cx, layout: &LayoutEngine, router: &mut EventRouter, x: f32, y: f32) {
    router.route(
        cx,
        layout,
        ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
            position: Point::new(x, y),
            delta: Point::new(0.0, 0.0),
            kind: PointerKind::Mouse,
            button: Some(MouseButton::Left),
            buttons: MouseButtons::LEFT,
            modifiers: ModifiersState::empty(),
            phase: PointerPhase::Down,
        })),
    );
}

#[test]
fn clicking_moves_the_caret_to_the_clicked_line() {
    sigil::reactive::Runtime::scope("caret-click", || {
        // Caret starts at the end of the buffer, on line 3 (index 2).
        sigil_layout::set_text_geometry(std::rc::Rc::new(sigil_app::EngineGeometry::new()));
        let mut cx = Cx::new_root();
        let root = cx.add(text_edit(signal(String::from("aaa\nbbb\nccc"))));
        let mut layout =
            LayoutEngine::with_text_measure(Box::new(sigil_app::DefaultTextMeasure::new()));
        layout
            .compute(&mut cx, root, Size::fixed(400.0, 200.0))
            .expect("layout");
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        layout
            .compute(&mut cx, root, Size::fixed(400.0, 200.0))
            .expect("relayout");

        let start = caret_rects(&cx, root).first().copied().expect("caret");
        // Click near the top-left (line 0). The caret must jump up there.
        click_at(&mut cx, &layout, &mut router, 15.0, 10.0);
        layout
            .compute(&mut cx, root, Size::fixed(400.0, 200.0))
            .expect("relayout after click");
        let after = caret_rects(&cx, root)
            .first()
            .copied()
            .expect("caret after click");
        assert!(
            after.origin.y + after.height < start.origin.y,
            "click on the first line should move the caret up from line 3; \
             start y={} after y={}",
            start.origin.y,
            after.origin.y,
        );
    });
}
