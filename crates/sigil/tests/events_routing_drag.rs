#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]
#![allow(clippy::float_cmp)]

#[path = "events_support/mock_shell.rs"]
mod mock_shell;
#[path = "events_support/routing_helpers.rs"]
mod routing_helpers;

use sigil::events::prelude::*;
use sigil::hal::{Point, PointerEventInit};
use sigil::kernel::prelude::*;
use sigil::layout::prelude::*;
use sigil::reactive::{Runtime, signal};

use routing_helpers::{pointer_down, route_event};

const fn pointer_move_delta(x: f32, y: f32, dx: f32, dy: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(dx, dy),
        kind: PointerKind::Mouse,
        button: None,
        buttons: MouseButtons::LEFT,
        modifiers: ModifiersState::empty(),
        phase: PointerPhase::Move,
    }))
}

fn node_rect(cx: &Cx, id: NodeId) -> sigil::hal::Rect {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("node has a layout result")
        .rect()
}

#[test]
#[allow(clippy::too_many_lines)]
fn dragging_a_position_reactive_node_moves_its_box() {
    Runtime::scope("events_routing_drag", || {
        let pos = signal((100.0_f32, 100.0_f32));

        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(800.0, 600.0))
                .child(
                    container()
                        .layout(LayoutStyle::flex_column().fixed_size(200.0, 80.0))
                        .position_reactive(pos)
                        .pointer_down(move |ev| {
                            ev.capture_pointer();
                            ev.stop_propagation();
                        })
                        .pointer_move(move |ev| {
                            if !ev.is_captured() {
                                return;
                            }
                            let (x, y) = pos.peek();
                            pos.set((x + ev.delta.x, y + ev.delta.y));
                            ev.stop_propagation();
                        }),
                ),
        );
        let card = *cx.get(root).unwrap().children().first().unwrap();

        let mut layout =
            LayoutEngine::with_text_measure(Box::new(routing_helpers::ZeroTextMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(800.0, 600.0))
            .unwrap();
        let start = node_rect(&cx, card);
        assert_eq!(
            (start.origin.x, start.origin.y),
            (100.0, 100.0),
            "card starts at the signal's initial inset",
        );

        let mut router = EventRouter::new();
        route_event(&mut router, &mut cx, &layout, pointer_down(150.0, 130.0));
        route_event(
            &mut router,
            &mut cx,
            &layout,
            pointer_move_delta(190.0, 160.0, 40.0, 30.0),
        );
        route_event(
            &mut router,
            &mut cx,
            &layout,
            pointer_move_delta(200.0, 155.0, 10.0, -5.0),
        );

        assert_eq!(
            pos.peek(),
            (150.0, 125.0),
            "captured moves accumulated both deltas into the position signal",
        );

        layout
            .compute(&mut cx, root, Size::fixed(800.0, 600.0))
            .unwrap();
        let moved = node_rect(&cx, card);
        assert_eq!(
            (moved.origin.x, moved.origin.y),
            (150.0, 125.0),
            "the card's box followed the drag to the new inset; got {:?}",
            (moved.origin.x, moved.origin.y),
        );

        let card_again = *cx.get(root).unwrap().children().first().unwrap();
        assert_eq!(card, card_again, "node identity stable across the drag");
    });
}
