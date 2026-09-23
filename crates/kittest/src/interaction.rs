use sigil::hal::{
    ModifiersState, MouseButton, MouseButtons, Point, PointerEventInit, PointerKind, PointerPhase,
    ShellEvent,
};
use sigil::prelude::*;

use super::{InteractionMode, SETTLE_SAFETY_CAP};

pub(crate) fn drive_state(
    router: &mut EventRouter,
    cx: &mut Cx,
    layout: &LayoutEngine,
    focus_target: NodeId,
    mode: InteractionMode,
    hit: (f32, f32),
) {
    match mode {
        InteractionMode::Default | InteractionMode::Disabled => {}
        InteractionMode::Hover | InteractionMode::HoverAfterFrames { .. } => {
            router.route(cx, layout, pointer_move(hit.0, hit.1));
        }
        InteractionMode::Pressed | InteractionMode::PressedAfterFrames { .. } => {
            router.route(cx, layout, pointer_move(hit.0, hit.1));
            router.route(cx, layout, pointer_down(hit.0, hit.1));
        }
        InteractionMode::Clicked => {
            router.route(cx, layout, pointer_move(hit.0, hit.1));
            router.route(cx, layout, pointer_down(hit.0, hit.1));
            router.route(cx, layout, pointer_up(hit.0, hit.1));
        }
        InteractionMode::Focused | InteractionMode::FocusedAfterFrames { .. } => {
            router.set_focus(cx, Some(focus_target));
        }
    }
}

pub(crate) fn advance(mode: InteractionMode, mut layout: impl FnMut()) {
    let (frames, dt) = match mode {
        InteractionMode::HoverAfterFrames { frames, dt }
        | InteractionMode::PressedAfterFrames { frames, dt }
        | InteractionMode::FocusedAfterFrames { frames, dt } => (frames, dt),
        InteractionMode::Hover
        | InteractionMode::Pressed
        | InteractionMode::Focused
        | InteractionMode::Clicked => (u32::MAX, 1.0 / 60.0),
        InteractionMode::Default | InteractionMode::Disabled => return,
    };
    if frames == u32::MAX {
        for _ in 0..SETTLE_SAFETY_CAP {
            if sigil::animation_runtime::active_count() == 0 {
                return;
            }
            sigil::animation_runtime::tick(dt);
            layout();
        }
        let remaining = sigil::animation_runtime::active_count();
        assert!(
            remaining == 0,
            "golden harness: animation runtime did not settle within \
             SETTLE_SAFETY_CAP ({SETTLE_SAFETY_CAP}) frames at dt={dt}. \
             Active animators remaining: {remaining}. A widget under test is \
             likely retargeting every frame.",
        );
        return;
    }
    for _ in 0..frames {
        sigil::animation_runtime::tick(dt);
        layout();
    }
}

fn pointer_move(x: f32, y: f32) -> ShellEvent {
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

fn pointer_down(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        phase: PointerPhase::Down,
        buttons: MouseButtons::LEFT,
        modifiers: ModifiersState::empty(),
    }))
}

fn pointer_up(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        phase: PointerPhase::Up,
        buttons: MouseButtons::empty(),
        modifiers: ModifiersState::empty(),
    }))
}
