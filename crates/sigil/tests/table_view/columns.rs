use super::*;
use sigil::events::EventRouter;
use sigil::events::ShellEvent;
use sigil::kernel::Cx;

const fn pointer_button(
    phase: PointerPhase,
    x: f32,
    y: f32,
    button: MouseButton,
    buttons: MouseButtons,
) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(button),
        buttons,
        modifiers: ModifiersState::empty(),
        phase,
    }))
}

#[test]
fn resize_handle_pins_column_width() {
    sigil::reactive::Runtime::scope("table-view-resize", || {
        let state = demo_state();
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state.clone())
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(VIEW_H),
        );
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();
        router.route(
            &mut cx,
            &layout,
            pointer_button(
                PointerPhase::Down,
                109.0,
                14.0,
                MouseButton::Left,
                MouseButtons::LEFT,
            ),
        );
        router.route(
            &mut cx,
            &layout,
            pointer_button(
                PointerPhase::Move,
                149.0,
                14.0,
                MouseButton::Left,
                MouseButtons::LEFT,
            ),
        );
        router.route(
            &mut cx,
            &layout,
            pointer_button(
                PointerPhase::Up,
                149.0,
                14.0,
                MouseButton::Left,
                MouseButtons::empty(),
            ),
        );
        assert_eq!(
            state.column_width("name"),
            Some(140.0),
            "drag +40 px pins the name column",
        );
    });
}

#[test]
fn right_click_row_selects() {
    sigil::reactive::Runtime::scope("table-view-context", || {
        let state = demo_state();
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state.clone())
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(VIEW_H),
        );
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();
        let row_y = ROW_H.mul_add(1.5, HEADER_H);
        router.route(
            &mut cx,
            &layout,
            pointer_button(
                PointerPhase::Down,
                50.0,
                row_y,
                MouseButton::Right,
                MouseButtons::RIGHT,
            ),
        );
        assert!(
            state.selection().get().contains(&1),
            "right-click selects the row",
        );
    });
}

#[test]
fn fraction_column_honours_min_width() {
    sigil::reactive::Runtime::scope("table-min-width", || {
        let state = TableState::new(
            demo_rows(),
            vec![
                TableColumn::new("name", "Name")
                    .width(ColWidth::Fixed(300.0))
                    .cell(|row: &Item| container().child(text(row.name.clone()))),
                TableColumn::new("count", "Count")
                    .width(ColWidth::Fraction(1.0))
                    .min_width(192.0)
                    .cell(|row: &Item| container().child(text(row.count.to_string()))),
            ],
        );
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state)
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(400.0)
                .viewport_height(VIEW_H),
        );
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let header = *cx
            .get(root)
            .expect("grid")
            .children()
            .first()
            .expect("header");
        let track = *cx
            .get(header)
            .expect("header row")
            .children()
            .first()
            .expect("header cell track");
        let count_cell = cx
            .get(track)
            .expect("track")
            .children()
            .get(1)
            .copied()
            .expect("count header");
        let width = layout.result(count_cell).expect("count rect").rect().width;
        assert!(
            width >= 192.0,
            "Fraction min_width(192) must hold in a 400px table beside Fixed(300), got {width}"
        );
    });
}
