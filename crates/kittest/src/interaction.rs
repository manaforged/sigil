use sigil::hal::{
    ModifiersState, MouseButton, MouseButtons, Point, PointerEventInit, PointerKind, PointerPhase,
    ShellEvent,
};
use sigil::prelude::*;

use super::{Error, InteractionMode, SETTLE_SAFETY_CAP};

pub(crate) fn drive_state(
    cx: &mut Cx,
    layout: &LayoutEngine,
    focus_target: NodeId,
    mode: InteractionMode,
    hit: (f32, f32),
) {
    let mut router = EventRouter::new();
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
        InteractionMode::Dragged { dx, dy } => {
            router.route(cx, layout, pointer_move(hit.0, hit.1));
            router.route(cx, layout, pointer_down(hit.0, hit.1));
            router.route(cx, layout, pointer_drag(hit.0 + dx, hit.1 + dy));
        }
    }
}

const fn pointer_drag(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: None,
        phase: PointerPhase::Move,
        buttons: MouseButtons::LEFT,
        modifiers: ModifiersState::empty(),
    }))
}

pub(crate) fn advance(
    mode: InteractionMode,
    mut layout: impl FnMut() -> Result<(), Error>,
) -> Result<(), Error> {
    let (frames, dt) = match mode {
        InteractionMode::HoverAfterFrames { frames, dt }
        | InteractionMode::PressedAfterFrames { frames, dt }
        | InteractionMode::FocusedAfterFrames { frames, dt } => (frames, dt),
        InteractionMode::Hover
        | InteractionMode::Pressed
        | InteractionMode::Focused
        | InteractionMode::Clicked
        | InteractionMode::Dragged { .. } => (u32::MAX, 1.0 / 60.0),
        InteractionMode::Default | InteractionMode::Disabled => return Ok(()),
    };
    if frames != u32::MAX {
        for _ in 0..frames {
            sigil::animation_runtime::tick(dt);
            layout()?;
        }
        return Ok(());
    }
    for _ in 0..SETTLE_SAFETY_CAP {
        if sigil::animation_runtime::active_count() == 0 {
            return Ok(());
        }
        sigil::animation_runtime::tick(dt);
        layout()?;
    }
    match sigil::animation_runtime::active_count() {
        0 => Ok(()),
        active => Err(Error::Unsettled {
            frames: SETTLE_SAFETY_CAP,
            active,
        }),
    }
}

const fn pointer_move(x: f32, y: f32) -> ShellEvent {
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

const fn pointer_down(x: f32, y: f32) -> ShellEvent {
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

const fn pointer_up(x: f32, y: f32) -> ShellEvent {
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
