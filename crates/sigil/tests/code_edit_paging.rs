#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::hal::{KeyEventInit, Point, PointerEventInit};
use sigil::prelude::*;
use sigil_kittest::FixedMeasure;

const TEXT_MEASURE: FixedMeasure = FixedMeasure {
    width: 80.0,
    height: 16.0,
};

const fn click(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        buttons: MouseButtons::LEFT,
        modifiers: ModifiersState::empty(),
        phase: PointerPhase::Down,
    }))
}

fn page_down() -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::Named(Key::PageDown),
        physical: PhysicalKey::PageDown,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

#[test]
fn page_down_moves_one_viewport_of_rows() {
    sigil::reactive::Runtime::scope("code-edit-page", || {
        const ROW_H: f32 = 20.0;
        const PANE_H: f32 = 300.0;
        let lines = signal((0..200).map(|i| format!("line {i}")).collect::<Vec<_>>());
        let caret = signal((0_usize, 0_usize));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(400.0, PANE_H))
                .child(code_edit(lines, caret).row_height(ROW_H)),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Size::fixed(400.0, PANE_H))
            .unwrap();
        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, click(5.0, 5.0));
        let start = caret.get().0;
        router.route(&mut cx, &layout, page_down());
        assert_eq!(caret.get().0 - start, 15, "300 px pane / 20 px rows");
    });
}
