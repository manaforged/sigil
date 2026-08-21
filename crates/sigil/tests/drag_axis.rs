//! `axis_drag` — capture-on-handle drag primitive, end-to-end through
//! the real event router: snapshot at down, deltas against the snapshot
//! at move, settle at up, no compounding across drags.

#![allow(missing_docs)]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::too_many_lines)]
// Exact layout math: the widths under test are integer pixel results, so
// asserting them with `==` is correct, not a float-precision hazard.
#![allow(clippy::float_cmp)]

use sigil::prelude::*;
use sigil_hal::{Point, PointerEventInit};

const ROW_H: f32 = 20.0;

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

struct StubMeasure;
impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

fn compute(cx: &mut Cx, root: NodeId) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout
        .compute(cx, root, Size::fixed(200.0, 100.0))
        .expect("layout");
    layout
}

#[test]
fn axis_drag_applies_delta_against_down_snapshot_and_resettles() {
    Runtime::scope("axis-drag-snapshot", || {
        let width = signal(100.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(build_handle(width));
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();

        // Drag 1: down at x=10, move to x=60 → width = 100 + 50.
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

        // Drag 2: fresh snapshot (150) — down at x=60, move to x=30 → 150 − 30.
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

        // Clamp: drag far left — width floors at 10.
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
