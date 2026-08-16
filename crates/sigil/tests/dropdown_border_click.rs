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
use sigil_renderer::{SceneCommand, StrokeAlign, emit_scene};

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

fn compute(cx: &mut Cx, root: NodeId) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    recompute(cx, root, &mut layout);
    layout
}

fn recompute(cx: &mut Cx, root: NodeId, layout: &mut LayoutEngine) {
    layout
        .compute(cx, root, Size::fixed(640.0, 480.0))
        .expect("layout");
}

fn pointer(x: f32, y: f32, phase: PointerPhase) -> ShellEvent {
    let up_or_down = matches!(phase, PointerPhase::Down | PointerPhase::Up);
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: up_or_down.then_some(MouseButton::Left),
        buttons: if phase == PointerPhase::Down {
            MouseButtons::LEFT
        } else {
            MouseButtons::empty()
        },
        modifiers: ModifiersState::empty(),
        phase,
    }))
}

fn click(router: &mut EventRouter, cx: &mut Cx, layout: &LayoutEngine, x: f32, y: f32) {
    router.route(cx, layout, pointer(x, y, PointerPhase::Move));
    router.route(cx, layout, pointer(x, y, PointerPhase::Down));
    router.route(cx, layout, pointer(x, y, PointerPhase::Up));
}

fn frame_event(
    router: &mut EventRouter,
    cx: &mut Cx,
    root: NodeId,
    layout: &mut LayoutEngine,
    event: ShellEvent,
) {
    Runtime::begin_effect();
    router.route(cx, layout, event);
    Runtime::begin_layout();
    if Runtime::frame_dirty() {
        recompute(cx, root, layout);
    }
    Runtime::begin_paint();
    Runtime::end_frame();
}

fn frame_click(
    router: &mut EventRouter,
    cx: &mut Cx,
    root: NodeId,
    layout: &mut LayoutEngine,
    position: Point,
) {
    for phase in [PointerPhase::Move, PointerPhase::Down, PointerPhase::Up] {
        frame_event(
            router,
            cx,
            root,
            layout,
            pointer(position.x, position.y, phase),
        );
    }
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

fn assert_single_bordered_trigger(cx: &Cx, root: NodeId, layout: &LayoutEngine) {
    let root_rect = layout.result(root).expect("root layout").rect();
    assert_eq!((root_rect.width, root_rect.height), (132.0, 28.0));

    let trigger = cx.nth_child(root, 0).expect("trigger");
    assert_eq!(
        cx.get(trigger).expect("trigger node").a11y.role,
        Role::Button,
        "the trigger must be the sole painted and hittable surface",
    );
    let trigger_rect = layout.result(trigger).expect("trigger layout").rect();
    assert_eq!(
        trigger_rect, root_rect,
        "trigger must fill the compact root"
    );

    let scene = emit_scene(cx, root);
    let trigger_rects: Vec<_> = scene
        .commands
        .iter()
        .filter_map(|command| match command {
            SceneCommand::Rect(rect) if rect.rect == trigger_rect => Some(rect),
            _ => None,
        })
        .collect();
    assert_eq!(
        trigger_rects.len(),
        1,
        "compact dropdown must paint one trigger surface, not nested overlapping frames",
    );
    let stroke = trigger_rects
        .first()
        .expect("one trigger surface")
        .stroke
        .expect("the trigger surface must paint a real border");
    assert!((stroke.width - 1.0).abs() < f32::EPSILON);
    assert_eq!(stroke.align, StrokeAlign::Inside);
}

#[test]
fn border_click_toggles_popup_open_and_closed() {
    sigil::reactive::Runtime::scope("dd-border-click", || {
        let mut cx = Cx::new_root();
        let root = dropdown_fixture(&mut cx);
        let mut layout = compute(&mut cx, root);
        let closed = glyph_runs(&mut cx, root);

        let mut router = EventRouter::new();
        // (0.5, 20) sits inside the 1 px frame, outside the padded
        // interior node — the exact dead zone from the gate finding.
        click(&mut router, &mut cx, &layout, 0.5, 20.0);
        recompute(&mut cx, root, &mut layout);
        let open = glyph_runs(&mut cx, root);
        assert_eq!(open, closed + 2, "border click must open the popup");

        click(&mut router, &mut cx, &layout, 0.5, 20.0);
        recompute(&mut cx, root, &mut layout);
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
        let mut layout = compute(&mut cx, root);
        let closed = glyph_runs(&mut cx, root);

        let mut router = EventRouter::new();
        // A double-fire (inner handler + frame handler) would toggle
        // open then immediately closed again.
        click(&mut router, &mut cx, &layout, 100.0, 20.0);
        recompute(&mut cx, root, &mut layout);
        assert_eq!(
            glyph_runs(&mut cx, root),
            closed + 2,
            "interior click must toggle exactly once (popup open)"
        );
    });
}

#[test]
fn trigger_reopens_after_selecting_an_option() {
    sigil::reactive::Runtime::scope("dd-reopen-after-select", || {
        let selected = signal(0_usize);
        let mut cx = Cx::new_root();
        let root = cx.add(dropdown(
            selected,
            ["Alpha", "Beta", "Gamma", "Delta", "Epsilon"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        ));
        let mut layout = compute(&mut cx, root);
        let closed = glyph_runs(&mut cx, root);
        let mut router = EventRouter::new();

        click(&mut router, &mut cx, &layout, 100.0, 20.0);
        recompute(&mut cx, root, &mut layout);
        // y=74 is the midpoint of option row 1 (popup top 32 + 28 + 14).
        click(&mut router, &mut cx, &layout, 100.0, 74.0);
        recompute(&mut cx, root, &mut layout);
        assert_eq!(selected.get(), 1, "second option must be selected");
        assert_eq!(
            glyph_runs(&mut cx, root),
            closed,
            "selection must close popup"
        );

        click(&mut router, &mut cx, &layout, 100.0, 20.0);
        recompute(&mut cx, root, &mut layout);
        assert_eq!(
            glyph_runs(&mut cx, root),
            closed + 5,
            "trigger must reopen popup after a selection",
        );
    });
}

#[test]
fn trigger_reopens_when_each_pointer_event_runs_in_its_own_frame() {
    sigil::reactive::Runtime::scope("dd-production-frame-reopen", || {
        let selected = signal(0_usize);
        let mut cx = Cx::new_root();
        let root = cx.add(dropdown(
            selected,
            ["Alpha", "Beta", "Gamma", "Delta", "Epsilon"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        ));
        let mut layout = compute(&mut cx, root);
        let closed = glyph_runs(&mut cx, root);
        let mut router = EventRouter::new();

        frame_click(
            &mut router,
            &mut cx,
            root,
            &mut layout,
            Point::new(100.0, 20.0),
        );
        // y=74 is the midpoint of option row 1 (popup top 32 + 28 + 14).
        frame_click(
            &mut router,
            &mut cx,
            root,
            &mut layout,
            Point::new(100.0, 74.0),
        );
        assert_eq!(selected.get(), 1, "second option must be selected");
        assert_eq!(glyph_runs(&mut cx, root), closed);

        frame_click(
            &mut router,
            &mut cx,
            root,
            &mut layout,
            Point::new(100.0, 20.0),
        );
        assert_eq!(
            glyph_runs(&mut cx, root),
            closed + 5,
            "production frame sequencing must preserve the popup subscription",
        );
    });
}

#[test]
fn compact_dropdown_keeps_its_label_and_popup_anchor() {
    sigil::reactive::Runtime::scope("dd-compact-geometry", || {
        let mut cx = Cx::new_root();
        let root = cx.add(
            dropdown(
                signal(0_usize),
                vec!["the reviewer".to_owned(), "the reviewer".to_owned()],
            )
            .label("Agent")
            .width(132.0)
            .height(28.0),
        );
        let mut layout = compute(&mut cx, root);
        assert!(
            matches!(&cx.get(root).expect("root").a11y.name, AccessibleName::Static(name) if name == "Agent")
        );
        assert_single_bordered_trigger(&cx, root, &layout);

        click(&mut EventRouter::new(), &mut cx, &layout, 66.0, 14.0);
        recompute(&mut cx, root, &mut layout);
        let popup = cx.nth_child(root, 1).expect("popup");
        let popup_y = layout
            .result(popup)
            .expect("open popup layout")
            .rect()
            .origin
            .y;
        assert!(
            (popup_y - 28.0).abs() < f32::EPSILON,
            "compact popup must anchor at y=28; got {popup_y}",
        );
    });
}
