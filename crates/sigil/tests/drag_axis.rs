#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::too_many_lines)]
#![allow(clippy::float_cmp)]

#[path = "widget_support/mod.rs"]
mod widget_support;

use sigil::hal::{Point, PointerEventInit};
use sigil::prelude::*;
use widget_support::compute;

const ROW_H: f32 = 20.0;
const VIEWPORT: Size = Size::fixed(200.0, 100.0);

const fn pointer(x: f32, y: f32, phase: PointerPhase, button: Option<MouseButton>) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button,
        phase,
        buttons: MouseButtons::empty(),
        modifiers: ModifiersState::empty(),
    }))
}

fn build_handle(width_signal: Signal<f32>) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(100.0, ROW_H))
        .child(sigil::widgets::axis_drag(
            container().layout(LayoutStyle::block().fixed_size(100.0, ROW_H)),
            sigil::widgets::Axis::X,
            move || width_signal.peek(),
            move |base, delta| width_signal.set((base + delta).max(10.0)),
            || {},
        ))
}

#[test]
fn axis_drag_applies_delta_against_down_snapshot_and_resettles() {
    Runtime::scope("axis-drag-snapshot", || {
        let width = signal(100.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(build_handle(width));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();

        router.route(
            &mut cx,
            &layout,
            pointer(10.0, 10.0, PointerPhase::Down, Some(MouseButton::Left)),
        );
        router.route(
            &mut cx,
            &layout,
            pointer(60.0, 10.0, PointerPhase::Move, None),
        );
        assert_eq!(width.get(), 150.0, "delta applies against down snapshot");

        router.route(
            &mut cx,
            &layout,
            pointer(60.0, 10.0, PointerPhase::Up, None),
        );
        router.route(
            &mut cx,
            &layout,
            pointer(60.0, 10.0, PointerPhase::Down, Some(MouseButton::Left)),
        );
        router.route(
            &mut cx,
            &layout,
            pointer(30.0, 10.0, PointerPhase::Move, None),
        );
        assert_eq!(
            width.get(),
            120.0,
            "second drag snapshots fresh state — no compounding"
        );

        router.route(
            &mut cx,
            &layout,
            pointer(30.0, 10.0, PointerPhase::Up, None),
        );
        router.route(
            &mut cx,
            &layout,
            pointer(30.0, 10.0, PointerPhase::Down, Some(MouseButton::Left)),
        );
        router.route(
            &mut cx,
            &layout,
            pointer(-500.0, 10.0, PointerPhase::Move, None),
        );
        assert_eq!(width.get(), 10.0, "caller clamp applies");
    });
}
