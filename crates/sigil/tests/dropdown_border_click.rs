//! Pointer routing over the dropdown trigger's 1 px rest frame.
//!
//! The frame is part of the hit surface: a click landing exactly on
//! the border must toggle the popup just like an interior click, and
//! an interior click must toggle exactly once (`release_pointer`
//! consumes the press, so the frame's mirrored handler cannot
//! double-fire). the reviewer gate finding, 2026-07-13.
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
use sigil_reactive as _;
use sigil_style as _;
use sigil_tasks as _;
use tiny_skia as _;
use wgpu as _;

use sigil::prelude::*;
use sigil_hal::{MouseButton, MouseButtons, Point, PointerEventInit};
use sigil_renderer::{SceneCommand, emit_scene};

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

fn compute(cx: &mut Cx, root: NodeId) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout
        .compute(cx, root, Size::fixed(640.0, 480.0))
        .expect("layout");
    layout
}

fn pointer(x: f32, y: f32, phase: PointerPhase) -> ShellEvent {
    let up_or_down = matches!(phase, PointerPhase::Down | PointerPhase::Up);
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: up_or_down.then_some(MouseButton::Left),
        buttons: MouseButtons::LEFT,
        modifiers: ModifiersState::empty(),
        phase,
    }))
}

fn click(router: &mut EventRouter, cx: &mut Cx, layout: &LayoutEngine, x: f32, y: f32) {
    router.route(cx, layout, pointer(x, y, PointerPhase::Down));
    router.route(cx, layout, pointer(x, y, PointerPhase::Up));
}

/// Glyph runs visible in the scene — 1 when closed (trigger label),
/// 1 + option count when the popup is open.
fn glyph_runs(cx: &mut Cx, root: NodeId) -> usize {
    compute(cx, root);
    emit_scene(cx, root)
        .commands
        .iter()
        .filter(|c| matches!(c, SceneCommand::GlyphRun(_)))
        .count()
}

fn dropdown_fixture(cx: &mut Cx) -> NodeId {
    cx.add(dropdown(
        signal(0_usize),
        vec!["Alpha".to_owned(), "Beta".to_owned()],
    ))
}

#[test]
fn border_click_toggles_popup_open_and_closed() {
    sigil::reactive::Runtime::scope("dd-border-click", || {
        let mut cx = Cx::new_root();
        let root = dropdown_fixture(&mut cx);
        let layout = compute(&mut cx, root);
        let closed = glyph_runs(&mut cx, root);

        let mut router = EventRouter::new();
        // (0.5, 20) sits inside the 1 px frame, outside the padded
        // interior node — the exact dead zone from the gate finding.
        click(&mut router, &mut cx, &layout, 0.5, 20.0);
        let open = glyph_runs(&mut cx, root);
        assert_eq!(open, closed + 2, "border click must open the popup");

        click(&mut router, &mut cx, &layout, 0.5, 20.0);
        assert_eq!(
            glyph_runs(&mut cx, root),
            closed,
            "second border click must close the popup"
        );
    });
}

/// A `disabled` flip between pointer-down and pointer-up must veto
/// activation on release — for the frame path AND the interior path
/// (the reviewer re-gate finding, 2026-07-13).
fn mid_gesture_disable_vetoes_release(scope: &'static str, x: f32, y: f32) {
    sigil::reactive::Runtime::scope(scope, || {
        let disabled = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(
            dropdown(signal(0_usize), vec!["Alpha".to_owned(), "Beta".to_owned()])
                .disabled(disabled),
        );
        let layout = compute(&mut cx, root);
        let closed = glyph_runs(&mut cx, root);

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer(x, y, PointerPhase::Down));
        disabled.set(true);
        router.route(&mut cx, &layout, pointer(x, y, PointerPhase::Up));
        assert_eq!(
            glyph_runs(&mut cx, root),
            closed,
            "release after mid-gesture disable must not open the popup"
        );
    });
}

#[test]
fn mid_gesture_disable_vetoes_border_release() {
    mid_gesture_disable_vetoes_release("dd-disable-border", 0.5, 20.0);
}

/// The keyboard open path must honor a reactive disable too: focus
/// while enabled, flip `disabled`, `ArrowDown` must not open the popup
/// (the reviewer re-gate round 3, 2026-07-13).
#[test]
fn reactive_disable_vetoes_arrow_down_open() {
    sigil::reactive::Runtime::scope("dd-disable-arrow", || {
        let disabled = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(
            dropdown(signal(0_usize), vec!["Alpha".to_owned(), "Beta".to_owned()])
                .disabled(disabled),
        );
        let layout = compute(&mut cx, root);
        let closed = glyph_runs(&mut cx, root);

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        disabled.set(true);
        router.route(
            &mut cx,
            &layout,
            ShellEvent::Key(KeyEvent::new(sigil_hal::KeyEventInit {
                key: Key::ArrowDown.into(),
                physical: sigil_hal::PhysicalKey::ArrowDown,
                modifiers: ModifiersState::empty(),
                repeat: false,
                phase: KeyPhase::Down,
                text: None,
            })),
        );
        assert_eq!(
            glyph_runs(&mut cx, root),
            closed,
            "ArrowDown on a reactively-disabled dropdown must not open the popup"
        );
    });
}

#[test]
fn mid_gesture_disable_vetoes_interior_release() {
    mid_gesture_disable_vetoes_release("dd-disable-interior", 100.0, 20.0);
}

#[test]
fn interior_click_toggles_exactly_once() {
    sigil::reactive::Runtime::scope("dd-interior-click", || {
        let mut cx = Cx::new_root();
        let root = dropdown_fixture(&mut cx);
        let layout = compute(&mut cx, root);
        let closed = glyph_runs(&mut cx, root);

        let mut router = EventRouter::new();
        // A double-fire (inner handler + frame handler) would toggle
        // open then immediately closed again.
        click(&mut router, &mut cx, &layout, 100.0, 20.0);
        assert_eq!(
            glyph_runs(&mut cx, root),
            closed + 2,
            "interior click must toggle exactly once (popup open)"
        );
    });
}
