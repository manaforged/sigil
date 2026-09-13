#![allow(missing_docs)]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::prelude::*;
use sigil::renderer::{SceneCommand, emit_scene};

fn caret_rects(cx: &Cx, root: NodeId) -> Vec<sigil::hal::Rect> {
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
    sigil::layout::set_text_geometry(std::rc::Rc::new(sigil::app::EngineGeometry::new()));

    let mut cx = Cx::new_root();
    let root = cx.add(text_edit(signal(buffer.to_owned())));
    let mut layout =
        LayoutEngine::with_text_measure(Box::new(sigil::app::DefaultTextMeasure::new()));
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
        let (cx, root, _layout) = focused_edit("aaa\nbbb\nccc");
        let carets = caret_rects(&cx, root);
        let caret = carets.first().expect("caret present");
        assert!(
            caret.origin.y > 8.0 + caret.height,
            "caret for an end-of-buffer offset on line 3 should be well below \
             the first line; got y={} h={}",
            caret.origin.y,
            caret.height,
        );
    });
}

use sigil::hal::{KeyEventInit, MouseButton, MouseButtons, Point, PointerEventInit};

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
        sigil::layout::set_text_geometry(std::rc::Rc::new(sigil::app::EngineGeometry::new()));
        let mut cx = Cx::new_root();
        let root = cx.add(text_edit(signal(String::from("aaa\nbbb\nccc"))));
        let mut layout =
            LayoutEngine::with_text_measure(Box::new(sigil::app::DefaultTextMeasure::new()));
        layout
            .compute(&mut cx, root, Size::fixed(400.0, 200.0))
            .expect("layout");
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        layout
            .compute(&mut cx, root, Size::fixed(400.0, 200.0))
            .expect("relayout");

        let start = caret_rects(&cx, root).first().copied().expect("caret");
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

#[test]
fn caller_owned_caret_tracks_keyboard_movement() {
    sigil::reactive::Runtime::scope("caret-signal", || {
        sigil::layout::set_text_geometry(std::rc::Rc::new(sigil::app::EngineGeometry::new()));
        let mut cx = Cx::new_root();
        let caret = signal(5);
        let root = cx.add(text_edit(signal("hello".to_string())).caret_position(caret));
        let mut layout =
            LayoutEngine::with_text_measure(Box::new(sigil::app::DefaultTextMeasure::new()));
        layout
            .compute(&mut cx, root, Size::fixed(400.0, 200.0))
            .expect("layout");
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            ShellEvent::Key(KeyEvent::new(KeyEventInit {
                key: LogicalKey::Named(Key::ArrowLeft),
                physical: PhysicalKey::Unidentified,
                modifiers: ModifiersState::empty(),
                repeat: false,
                phase: KeyPhase::Down,
                text: None,
            })),
        );

        assert_eq!(caret.peek(), 4);
    });
}
