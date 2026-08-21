//! Terminal mouse reporting, mapped onto the shared pointer vocabulary.
//!
//! A terminal reports the cell a pointer is over. Widgets hit-test in logical
//! pixels, so a cell becomes the point at its centre — not its corner. A
//! corner sits exactly on a boundary, and any sub-cell offset in the layout
//! (a one-pixel border is enough) puts it in the box above or to the left.

use crossterm::event::{KeyModifiers, MouseButton as TermButton, MouseEvent, MouseEventKind};
use sigil_hal::PointerEvent;
use sigil_hal::{
    ModifiersState, MouseButton, MouseButtons, Point, PointerEventInit, PointerKind, PointerPhase,
    ScrollDelta, ScrollEvent, ScrollEventInit, ScrollPhase, ShellEvent,
};

/// Lines a wheel notch scrolls. Terminals report notches, not pixels.
const NOTCH: f32 = 3.0;

/// Translate a terminal mouse report into a shell event.
///
/// `cell` is the px-per-cell scale layout ran at. `last` carries the previous
/// position so a move reports how far it moved.
#[must_use]
pub fn map_mouse(event: &MouseEvent, cell: f32, last: &mut Point) -> Option<ShellEvent> {
    let at = Point::new(
        (f32::from(event.column) + 0.5) * cell,
        (f32::from(event.row) + 0.5) * cell,
    );
    let delta = Point::new(at.x - last.x, at.y - last.y);
    *last = at;
    let modifiers = modifiers(event.modifiers);

    let (phase, button) = match event.kind {
        MouseEventKind::Down(button) => (PointerPhase::Down, Some(button)),
        MouseEventKind::Up(button) => (PointerPhase::Up, Some(button)),
        MouseEventKind::Drag(button) => (PointerPhase::Move, Some(button)),
        MouseEventKind::Moved => (PointerPhase::Move, None),
        MouseEventKind::ScrollUp => return Some(scroll(at, 0.0, NOTCH, modifiers)),
        MouseEventKind::ScrollDown => return Some(scroll(at, 0.0, -NOTCH, modifiers)),
        MouseEventKind::ScrollLeft => return Some(scroll(at, NOTCH, 0.0, modifiers)),
        MouseEventKind::ScrollRight => return Some(scroll(at, -NOTCH, 0.0, modifiers)),
    };

    let button = button.map(press);
    // A drag holds its button down; a release no longer does.
    let held = match phase {
        PointerPhase::Up => MouseButtons::empty(),
        _ => button.map_or_else(MouseButtons::empty, bit),
    };
    Some(ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: at,
        delta,
        kind: PointerKind::Mouse,
        button,
        buttons: held,
        modifiers,
        phase,
    })))
}

const fn scroll(at: Point, x: f32, y: f32, modifiers: ModifiersState) -> ShellEvent {
    ShellEvent::Scroll(ScrollEvent::new(ScrollEventInit {
        position: at,
        delta: ScrollDelta::Lines(Point::new(x, y)),
        modifiers,
        phase: ScrollPhase::Moved,
    }))
}

const fn press(button: TermButton) -> MouseButton {
    match button {
        TermButton::Left => MouseButton::Left,
        TermButton::Right => MouseButton::Right,
        TermButton::Middle => MouseButton::Middle,
    }
}

const fn bit(button: MouseButton) -> MouseButtons {
    match button {
        MouseButton::Left => MouseButtons::LEFT,
        MouseButton::Right => MouseButtons::RIGHT,
        MouseButton::Middle => MouseButtons::MIDDLE,
        MouseButton::Back => MouseButtons::BACK,
        MouseButton::Forward => MouseButtons::FORWARD,
        // A terminal reports three buttons; anything else cannot arrive here.
        MouseButton::Other(_) => MouseButtons::EMPTY,
    }
}

fn modifiers(from: KeyModifiers) -> ModifiersState {
    let mut state = ModifiersState::empty();
    if from.contains(KeyModifiers::SHIFT) {
        state |= ModifiersState::SHIFT;
    }
    if from.contains(KeyModifiers::CONTROL) {
        state |= ModifiersState::CONTROL;
    }
    if from.contains(KeyModifiers::ALT) {
        state |= ModifiersState::ALT;
    }
    state
}
