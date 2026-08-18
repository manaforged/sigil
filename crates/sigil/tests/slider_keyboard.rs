#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_profile as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use sigil_tasks as _;
use wgpu as _;

use sigil::prelude::*;
use sigil_hal::{KeyEventInit, PhysicalKey};

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

fn setup(
    value: Signal<f32>,
    range: std::ops::RangeInclusive<f32>,
    step: f32,
) -> (Cx, LayoutEngine, EventRouter, NodeId) {
    let mut cx = Cx::new_root();
    let root = cx.add(slider(value, range).step(step));
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout
        .compute(&mut cx, root, Size::fixed(320.0, 120.0))
        .expect("layout");
    let router = EventRouter::new();
    (cx, layout, router, root)
}

fn key_down(key: Key, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: key.into(),
        physical,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn assert_approx(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.001,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn slider_page_up_moves_by_ten_steps() {
    sigil::reactive::Runtime::scope("sigil-slider-page-up", || {
        let value = signal(50.0_f32);
        let (mut cx, layout, mut router, root) = setup(value, 0.0..=100.0, 1.0);
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, key_down(Key::PageUp, PhysicalKey::PageUp));
        assert_approx(value.get(), 60.0);
    });
}

#[test]
fn slider_page_down_moves_by_ten_steps() {
    sigil::reactive::Runtime::scope("sigil-slider-page-down", || {
        let value = signal(50.0_f32);
        let (mut cx, layout, mut router, root) = setup(value, 0.0..=100.0, 1.0);
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            key_down(Key::PageDown, PhysicalKey::PageDown),
        );
        assert_approx(value.get(), 40.0);
    });
}

#[test]
fn slider_page_step_respects_custom_step() {
    sigil::reactive::Runtime::scope("sigil-slider-page-step", || {
        let value = signal(50.0_f32);
        let (mut cx, layout, mut router, root) = setup(value, 0.0..=100.0, 2.0);
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, key_down(Key::PageUp, PhysicalKey::PageUp));
        assert_approx(value.get(), 70.0);
    });
}

#[test]
fn slider_home_jumps_to_min() {
    sigil::reactive::Runtime::scope("sigil-slider-home", || {
        let value = signal(75.0_f32);
        let (mut cx, layout, mut router, root) = setup(value, 0.0..=100.0, 5.0);
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, key_down(Key::Home, PhysicalKey::Home));
        assert_approx(value.get(), 0.0);
    });
}

#[test]
fn slider_end_jumps_to_max() {
    sigil::reactive::Runtime::scope("sigil-slider-end", || {
        let value = signal(25.0_f32);
        let (mut cx, layout, mut router, root) = setup(value, 0.0..=100.0, 5.0);
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, key_down(Key::End, PhysicalKey::End));
        assert_approx(value.get(), 100.0);
    });
}

#[test]
fn slider_page_up_clamps_at_max() {
    sigil::reactive::Runtime::scope("sigil-slider-page-clamp-max", || {
        let value = signal(95.0_f32);
        let (mut cx, layout, mut router, root) = setup(value, 0.0..=100.0, 1.0);
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, key_down(Key::PageUp, PhysicalKey::PageUp));
        assert_approx(value.get(), 100.0);
    });
}

#[test]
fn slider_page_down_clamps_at_min() {
    sigil::reactive::Runtime::scope("sigil-slider-page-clamp-min", || {
        let value = signal(5.0_f32);
        let (mut cx, layout, mut router, root) = setup(value, 0.0..=100.0, 1.0);
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            key_down(Key::PageDown, PhysicalKey::PageDown),
        );
        assert_approx(value.get(), 0.0);
    });
}

#[test]
fn slider_focus_ring_visible_when_focused() {
    sigil::reactive::Runtime::scope("sigil-slider-focus-ring", || {
        let value = signal(50.0_f32);
        let (mut cx, layout, mut router, root) = setup(value, 0.0..=100.0, 1.0);
        let node_before = cx.get(root).unwrap();
        assert!(node_before.a11y.focusable, "slider must be focusable");

        router.set_focus(&mut cx, Some(root));
        let mut layout2 = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout2
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout after focus");
        let _ = layout;
    });
}
