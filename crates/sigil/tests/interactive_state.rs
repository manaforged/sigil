//! 5/5 Wave 1 (T2) — `interactive()` container wrapper, input→pixel proof.
//!
//! A plain container wrapped in `interactive()` must paint its
//! `.hover()` / `.pressed()` overlay slices when the router delivers
//! pointer events — ending the hello-style era of hand-rolled pointer
//! math. Also pins the non-propagating hover contract (ancestors keep
//! their own hover) and disabled inertness.

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
use sigil_tasks as _;
use wgpu as _;

use sigil::prelude::*;
use sigil_hal::{Point, PointerEventInit};
use sigil_renderer::{SceneCommand, emit_scene};
use sigil_style::Rgba8;

const HOVER_BG: Rgba8 = Rgba8 {
    r: 10,
    g: 200,
    b: 30,
    a: 255,
};
const PRESSED_BG: Rgba8 = Rgba8 {
    r: 200,
    g: 20,
    b: 200,
    a: 255,
};
const REST_BG: Rgba8 = Rgba8 {
    r: 240,
    g: 240,
    b: 240,
    a: 255,
};

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

fn rect_colors(cx: &Cx, root: NodeId) -> Vec<Rgba8> {
    emit_scene(cx, root)
        .commands
        .iter()
        .filter_map(|c| match c {
            SceneCommand::Rect(r) => Some(r.color),
            _ => None,
        })
        .collect()
}

const fn pointer_move_event(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: None,
        phase: PointerPhase::Move,
        buttons: MouseButtons::empty(),
        modifiers: ModifiersState::empty(),
    }))
}

const fn pointer_down_event(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(sigil_hal::MouseButton::Left),
        phase: PointerPhase::Down,
        buttons: MouseButtons::empty(),
        modifiers: ModifiersState::empty(),
    }))
}

fn rest_pane() -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
        .style(Style::new().bg(REST_BG))
}

#[test]
fn interactive_hover_overlay_paints_on_pointer_enter() {
    Runtime::scope("interactive-hover-paints", || {
        let mut cx = Cx::new_root();
        let root = cx.add(interactive(rest_pane()).hover(|s| s.bg(HOVER_BG)));
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();

        assert!(
            !rect_colors(&cx, root).contains(&HOVER_BG),
            "rest state must not paint the hover slice"
        );

        router.route(&mut cx, &layout, pointer_move_event(50.0, 50.0));
        assert!(
            rect_colors(&cx, root).contains(&HOVER_BG),
            "pointer enter must paint the .hover() slice"
        );

        router.route(&mut cx, &layout, pointer_move_event(300.0, 300.0));
        assert!(
            !rect_colors(&cx, root).contains(&HOVER_BG),
            "pointer leave must return to rest"
        );
    });
}

#[test]
fn interactive_pressed_overlay_paints_on_pointer_down() {
    Runtime::scope("interactive-pressed-paints", || {
        let mut cx = Cx::new_root();
        let root = cx.add(
            interactive(rest_pane())
                .hover(|s| s.bg(HOVER_BG))
                .pressed(|s| s.bg(PRESSED_BG)),
        );
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();

        router.route(&mut cx, &layout, pointer_move_event(50.0, 50.0));
        assert!(rect_colors(&cx, root).contains(&HOVER_BG));

        // Pressed slice replaces the hover slice while the button is down.
        router.route(&mut cx, &layout, pointer_down_event(50.0, 50.0));
        let colors = rect_colors(&cx, root);
        assert!(
            colors.contains(&PRESSED_BG),
            "pressed slice must paint on pointer down, got {colors:?}"
        );
        assert!(
            !colors.contains(&HOVER_BG),
            "pressed slice must supersede the hover slice"
        );
    });
}

#[test]
fn interactive_hover_does_not_stop_ancestor_hover() {
    Runtime::scope("interactive-non-propagating", || {
        let mut cx = Cx::new_root();
        let inner = interactive(rest_pane()).hover(|s| s.bg(HOVER_BG));
        let root = cx.add(
            interactive(
                container()
                    .layout(LayoutStyle::block().fixed_size(200.0, 200.0))
                    .style(Style::new().bg(REST_BG))
                    .child(inner),
            )
            .hover(|s| s.bg(HOVER_BG)),
        );
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_move_event(50.0, 50.0));
        // Both ancestor and descendant paint hover: the inner handler
        // must not stop propagation to the outer.
        let count = rect_colors(&cx, root)
            .iter()
            .filter(|c| **c == HOVER_BG)
            .count();
        assert_eq!(count, 2, "both nested interactive nodes must paint hover");
    });
}

#[test]
fn interactive_disabled_signal_suppresses_hover() {
    Runtime::scope("interactive-disabled-inert", || {
        let mut cx = Cx::new_root();
        let disabled = signal(true);
        let root = cx.add(
            interactive(rest_pane())
                .disabled_signal(disabled)
                .hover(|s| s.bg(HOVER_BG)),
        );
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_move_event(50.0, 50.0));
        assert!(
            !rect_colors(&cx, root).contains(&HOVER_BG),
            "disabled interactive must not paint hover"
        );
    });
}
