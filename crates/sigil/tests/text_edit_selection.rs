//! `text_edit` paints a multi-line selection highlight.
//!
//! Before this, `text_edit` had no selection overlay (the runtime comment
//! said "no selection overlay or scroll signal"). A selection spanning
//! lines paints one highlight rect per covered line — partial on the first
//! and last, full between. Verified against the reviewer's review.
//!
//! Driven here by keyboard shift-select, which flows through the buffer's
//! `shift_*` + `commit` (no pointer capture). Drag-select reuses the same
//! selection signal + overlay and is a follow-up (it needs the pointer
//! handlers on a hit-target node, as `text_field` does).

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
use sigil_bidi as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_profile as _;
use sigil_reactive as _;
use sigil_style as _;
use sigil_tasks as _;
use wgpu as _;

use sigil::prelude::*;
use sigil_hal::{KeyEventInit, PhysicalKey};
use sigil_renderer::{SceneCommand, emit_scene};

const SEL: sigil_style::Rgba8 = sigil_style::Rgba8 {
    r: 56,
    g: 118,
    b: 224,
    a: 130,
};

fn selection_ys(cx: &Cx, root: NodeId) -> Vec<f32> {
    let mut ys: Vec<f32> = emit_scene(cx, root)
        .commands
        .iter()
        .filter_map(|c| match c {
            SceneCommand::Rect(r) if r.color == SEL => Some(r.rect.origin.y),
            _ => None,
        })
        .collect();
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ys
}

fn shift_up() -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: Key::ArrowUp.into(),
        physical: PhysicalKey::ArrowUp,
        modifiers: ModifiersState::SHIFT,
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

#[test]
fn shift_select_across_lines_highlights_each_covered_line() {
    sigil::reactive::Runtime::scope("sel-kbd", || {
        sigil_layout::set_text_geometry(std::rc::Rc::new(sigil_app::EngineGeometry::new()));
        let mut cx = Cx::new_root();
        // Caret starts at the end of the buffer, on line 3 (index 2).
        let root = cx.add(text_edit(signal(String::from("aaaa\nbbbb\ncccc"))));
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

        assert!(selection_ys(&cx, root).is_empty(), "no selection at rest");

        // Shift+Up twice: extend the selection from line 3 up to line 1.
        router.route(&mut cx, &layout, shift_up());
        router.route(&mut cx, &layout, shift_up());
        layout
            .compute(&mut cx, root, Size::fixed(400.0, 200.0))
            .expect("relayout after select");

        let ys = selection_ys(&cx, root);
        assert_eq!(
            ys.len(),
            3,
            "a selection from line 3 up to line 1 must highlight all three lines, got {ys:?}",
        );
        assert!(
            ys.windows(2).all(|w| matches!(w, [a, b] if a < b)),
            "highlight rects step down line by line, got {ys:?}",
        );
    });
}

use sigil_hal::{MouseButton, MouseButtons, Point, PointerEventInit};

fn pt(x: f32, y: f32, phase: PointerPhase, buttons: MouseButtons) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: matches!(phase, PointerPhase::Down).then_some(MouseButton::Left),
        buttons,
        modifiers: ModifiersState::empty(),
        phase,
    }))
}

#[test]
fn dragging_across_lines_highlights_each_covered_line() {
    sigil::reactive::Runtime::scope("sel-drag", || {
        sigil_layout::set_text_geometry(std::rc::Rc::new(sigil_app::EngineGeometry::new()));
        let mut cx = Cx::new_root();
        let root = cx.add(text_edit(signal(String::from("aaaa\nbbbb\ncccc"))));
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

        router.route(
            &mut cx,
            &layout,
            pt(15.0, 12.0, PointerPhase::Down, MouseButtons::LEFT),
        );
        router.route(
            &mut cx,
            &layout,
            pt(60.0, 52.0, PointerPhase::Move, MouseButtons::LEFT),
        );
        layout
            .compute(&mut cx, root, Size::fixed(400.0, 200.0))
            .expect("relayout drag");

        let ys = selection_ys(&cx, root);
        assert_eq!(
            ys.len(),
            3,
            "drag from line 0 into line 2 highlights three lines, got {ys:?}"
        );
    });
}
