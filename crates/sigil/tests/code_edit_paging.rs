#![allow(clippy::unwrap_used)]

use sigil::events::EventRouter;
use sigil::events::PhysicalKey;
use sigil::events::{Event, KeyPhase, MouseButtons, PointerPhase};
use sigil::geometry::{Constraint, Point};
use sigil::kernel::Cx;
use sigil::layout::{LayoutEngine, LayoutStyle};
use sigil::prelude::*;
use sigil_kittest::FakeMeasure;

const TEXT_MEASURE: FakeMeasure = FakeMeasure::fixed(80.0, 16.0);

const fn click(x: f32, y: f32) -> Event {
    Event::pointer(PointerPhase::Down, Point::new(x, y))
        .button(Some(MouseButton::Left))
        .buttons(MouseButtons::LEFT)
}

fn page_down() -> Event {
    Event::key(Key::Named(NamedKey::PageDown), KeyPhase::Down).physical(PhysicalKey::PageDown)
}

#[test]
fn page_down_moves_one_viewport_of_rows() {
    sigil::reactive::Owner::scope("code-edit-page", || {
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
        let mut layout = LayoutEngine::new(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Constraint::fixed(400.0, PANE_H))
            .unwrap();
        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, click(5.0, 5.0));
        let start = caret.get().0;
        router.route(&mut cx, &layout, page_down());
        assert_eq!(caret.get().0 - start, 15, "300 px pane / 20 px rows");
    });
}
