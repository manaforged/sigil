#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
// Integration tests intentionally keep assertions direct and use indexing
// for fixed test topology. See the hygiene framework.
#![allow(clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]
#![allow(clippy::float_cmp)]

//! End-to-end drag → relayout for a `position_reactive` node.
//!
//! This is the exact chain the Workflows node-graph relies on
//! (the workflows-graph pattern): pointer-down captures the node,
//! each pointer-move accumulates `ev.delta` into a `Signal<(f32, f32)>`
//! bound via `position_reactive`, and the layout engine repositions the
//! node on the next `compute`. It exercises every link the unit tests
//! cover only in isolation:
//!   * capture binds to the requesting node (`router.rs` capture rule),
//!   * captured pointer-moves carry a real `delta` and `is_captured()`,
//!   * `pos.set` invalidates layout through the signature cache
//!     (`cache_key.rs`) with no dirty mark / generation bump,
//!   * the node's box actually moves by the dragged delta.
//!
//! Settles the open "does `position_reactive` relayout live during a
//! drag?" question with real routed events instead of a hand-driven
//! `compute` (the `layout` unit test already covers the bare-`set` case).

#[path = "support/mock_shell.rs"]
mod mock_shell;
#[path = "support/routing_helpers.rs"]
mod routing_helpers;

use sigil_events::prelude::*;
use sigil_hal::{Point, PointerEventInit};
use sigil_kernel::prelude::*;
use sigil_layout::prelude::*;
use sigil_reactive::{Runtime, signal};

use routing_helpers::{pointer_down, route_event};

/// A pointer-move carrying an explicit per-frame `delta`. The shared
/// `routing_helpers::pointer_move` hard-codes `delta = (0,0)` (fine for
/// hover/hit tests); a drag needs the real movement the winit shell
/// would compute in `push_cursor_move`.
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

fn node_rect(cx: &Cx, id: NodeId) -> sigil_hal::Rect {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("node has a layout result")
        .rect()
}

#[test]
// Upstream test walks one drag scenario end-to-end (68 lines); splitting
// the press/move/release phases would obscure it.
#[allow(clippy::too_many_lines)]
fn dragging_a_position_reactive_node_moves_its_box() {
    Runtime::scope("events_routing_drag", || {
        let pos = signal((100.0_f32, 100.0_f32));

        let mut cx = Cx::new_root();
        // A canvas (relative) holding one draggable card, mirroring
        // `workflows_graph::node_card`: capture on down, accumulate delta
        // into `pos` on captured moves.
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
        // Press inside the card (its box is [100,100]..[300,180]) → capture.
        route_event(&mut router, &mut cx, &layout, pointer_down(150.0, 130.0));
        // Drag: two moves accumulating (40,30) then (10,-5) → net (50,25).
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

        // The frame loop recomputes layout every frame; do the same here.
        // No node was marked dirty and the mutation generation never moved
        // — only the signature-hashed live position changed.
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

        // Identity is stable across the drag — capture survived, no rebuild.
        let card_again = *cx.get(root).unwrap().children().first().unwrap();
        assert_eq!(card, card_again, "node identity stable across the drag");
    });
}
