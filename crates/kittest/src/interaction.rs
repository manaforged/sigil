use sigil::prelude::*;

use super::{Error, InteractionMode};
use crate::input::{click, pointer_down, pointer_drag, pointer_move};

pub(crate) fn drive_state(
    cx: &mut Cx,
    layout: &LayoutEngine,
    focus_target: NodeId,
    mode: InteractionMode,
    hit: (f32, f32),
) {
    let mut router = EventRouter::new();
    let (x, y) = hit;
    let events: Vec<ShellEvent> = match mode {
        InteractionMode::Default | InteractionMode::Disabled => Vec::new(),
        InteractionMode::Hover | InteractionMode::HoverAfterFrames { .. } => {
            vec![pointer_move(x, y)]
        }
        InteractionMode::Pressed | InteractionMode::PressedAfterFrames { .. } => {
            vec![pointer_move(x, y), pointer_down(x, y)]
        }
        InteractionMode::Clicked => click(x, y).to_vec(),
        InteractionMode::Focused | InteractionMode::FocusedAfterFrames { .. } => {
            router.set_focus(cx, Some(focus_target));
            Vec::new()
        }
        InteractionMode::Dragged { dx, dy } => vec![
            pointer_move(x, y),
            pointer_down(x, y),
            pointer_drag(x + dx, y + dy),
        ],
    };
    for event in events {
        router.route(cx, layout, event);
    }
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
    let mut frame = || {
        sigil::animation_runtime::tick(dt);
        layout()
    };
    if frames != u32::MAX {
        for _ in 0..frames {
            frame()?;
        }
        return Ok(());
    }
    crate::settle::settle(|| sigil::animation_runtime::active_count() > 0, frame)
}
