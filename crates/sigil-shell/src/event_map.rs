//! Conversion helpers from winit events to Sigil HAL events.
//!
//! Key mapping lives in [`crate::key_map`]. Everything else — modifier
//! state, mouse buttons, scroll, touch, and coordinate conversion —
//! lives here.

#![allow(clippy::redundant_pub_crate)]

use sigil_hal::{
    ModifiersState, MouseButton, MouseButtons, Point, PointerPhase, ScrollDelta, ScrollPhase,
};
use winit::{
    event::{
        MouseButton as WinitMouseButton, MouseScrollDelta as WinitMouseScrollDelta, TouchPhase,
    },
    keyboard::ModifiersState as WinitModifiersState,
};

pub(crate) fn map_modifiers(modifiers: WinitModifiersState) -> ModifiersState {
    let mut out = ModifiersState::empty();
    out.set(ModifiersState::SHIFT, modifiers.shift_key());
    out.set(ModifiersState::CONTROL, modifiers.control_key());
    out.set(ModifiersState::ALT, modifiers.alt_key());
    out.set(ModifiersState::META, modifiers.super_key());
    out
}

pub(crate) const fn map_mouse_button(button: WinitMouseButton) -> MouseButton {
    match button {
        WinitMouseButton::Left => MouseButton::Left,
        WinitMouseButton::Right => MouseButton::Right,
        WinitMouseButton::Middle => MouseButton::Middle,
        WinitMouseButton::Back => MouseButton::Back,
        WinitMouseButton::Forward => MouseButton::Forward,
        WinitMouseButton::Other(value) => MouseButton::Other(value),
    }
}

pub(crate) const fn mouse_button_bit(button: MouseButton) -> Option<MouseButtons> {
    match button {
        MouseButton::Left => Some(MouseButtons::LEFT),
        MouseButton::Right => Some(MouseButtons::RIGHT),
        MouseButton::Middle => Some(MouseButtons::MIDDLE),
        MouseButton::Back => Some(MouseButtons::BACK),
        MouseButton::Forward => Some(MouseButtons::FORWARD),
        MouseButton::Other(_) => None,
    }
}

pub(crate) const fn map_touch_pointer_phase(phase: TouchPhase) -> PointerPhase {
    match phase {
        TouchPhase::Started => PointerPhase::Down,
        TouchPhase::Moved => PointerPhase::Move,
        TouchPhase::Ended => PointerPhase::Up,
        TouchPhase::Cancelled => PointerPhase::Cancel,
    }
}

pub(crate) const fn map_scroll_phase(phase: TouchPhase) -> ScrollPhase {
    match phase {
        TouchPhase::Started => ScrollPhase::Started,
        TouchPhase::Moved => ScrollPhase::Moved,
        TouchPhase::Ended => ScrollPhase::Ended,
        TouchPhase::Cancelled => ScrollPhase::Cancelled,
    }
}

pub(crate) const fn map_scroll_delta(delta: WinitMouseScrollDelta) -> ScrollDelta {
    match delta {
        WinitMouseScrollDelta::LineDelta(x, y) => ScrollDelta::Lines(Point::new(x, y)),
        WinitMouseScrollDelta::PixelDelta(delta) => {
            ScrollDelta::Pixels(point_from_f64(delta.x, delta.y))
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) const fn point_from_f64(x: f64, y: f64) -> Point {
    Point::new(x as f32, y as f32)
}
