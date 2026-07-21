//! A slider's pointer->value math must divide by the track's MEASURED
//! extent, not by its default constant.
//!
//! `set_from_position` used to divide by `HORIZONTAL_WIDTH` (180). Any
//! slider laid out at another width mapped clicks to the wrong value and
//! saturated early on drag, and nothing in the suite caught it because
//! every slider test used the default size.
//!
//! Surfaced by the Kimi production-readiness review (2026-07-20).

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
use sigil_app as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use sigil_tasks as _;
use wgpu as _;

use sigil::prelude::*;
use sigil_hal::{Point, PointerEventInit};

struct StubMeasure;
impl TextMeasure for StubMeasure {
    fn measure(&self, _c: &TextContent, _s: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(0.0, 0.0)
    }
}

fn click_at(x: f32, width: f32) -> f32 {
    let mut result = 0.0;
    sigil::reactive::Runtime::scope("slider-extent", || {
        let value = signal(0.0_f32);
        let mut cx = Cx::new_root();
        // Force the slider to a known non-default width.
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_row().fixed_size(width, 40.0))
                .child(slider(value, 0.0..=100.0).length(width)),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(width, 40.0))
            .expect("layout");

        let mut router = EventRouter::new();
        router.route(
            &mut cx,
            &layout,
            ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
                position: Point::new(x, 20.0),
                delta: Point::new(0.0, 0.0),
                kind: PointerKind::Mouse,
                button: Some(MouseButton::Left),
                buttons: MouseButtons::LEFT,
                modifiers: ModifiersState::empty(),
                phase: PointerPhase::Down,
            })),
        );
        result = value.peek();
    });
    result
}

#[test]
fn click_maps_through_the_measured_track_width() {
    // Midpoint of a 400px slider is 200px -> 50, regardless of the
    // 180px default. Dividing by the constant would give 200/180 -> 100.
    let mid = click_at(200.0, 400.0);
    assert!(
        (mid - 50.0).abs() < 1.0,
        "click at the midpoint of a 400px slider should be ~50, got {mid}",
    );
}

/// The point of fill-by-default: no `.length()` call, the track simply
/// takes the parent's width, and pointer mapping follows it.
#[test]
fn filled_slider_maps_clicks_through_its_parent_width() {
    let mut got = 0.0;
    sigil::reactive::Runtime::scope("slider-fill", || {
        let value = signal(0.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_row().fixed_size(400.0, 40.0))
                .child(slider(value, 0.0..=100.0)),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(400.0, 40.0))
            .expect("layout");
        let mut router = EventRouter::new();
        router.route(
            &mut cx,
            &layout,
            ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
                position: Point::new(300.0, 20.0),
                delta: Point::new(0.0, 0.0),
                kind: PointerKind::Mouse,
                button: Some(MouseButton::Left),
                buttons: MouseButtons::LEFT,
                modifiers: ModifiersState::empty(),
                phase: PointerPhase::Down,
            })),
        );
        got = value.peek();
    });
    // 300 of a filled 400px track is 75%, NOT 300/180 saturated to 100.
    assert!(
        (got - 75.0).abs() < 1.0,
        "click at 300px of a filled 400px slider should be ~75, got {got}",
    );
}

#[test]
fn click_maps_through_a_narrow_track() {
    // 100px slider: 25px in is 25%. Dividing by 180 would give ~14.
    let quarter = click_at(25.0, 100.0);
    assert!(
        (quarter - 25.0).abs() < 2.0,
        "click 25px into a 100px slider should be ~25, got {quarter}",
    );
}
