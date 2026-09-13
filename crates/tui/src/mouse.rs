use crossterm::event::{KeyModifiers, MouseButton as TermButton, MouseEvent, MouseEventKind};
use sigil::hal::PointerEvent;
use sigil::hal::{
    ModifiersState, MouseButton, MouseButtons, Point, PointerEventInit, PointerKind, PointerPhase,
    ScrollDelta, ScrollEvent, ScrollEventInit, ScrollPhase, ShellEvent,
};

const NOTCH: f32 = 3.0;

#[must_use]
pub fn cell_point(col: u16, row: u16) -> Point {
    let cell = sigil::app::Host::CELL;
    Point::new((f32::from(col) + 0.5) * cell, (f32::from(row) + 0.5) * cell)
}

#[must_use]
pub fn pointer(phase: PointerPhase, col: u16, row: u16) -> ShellEvent {
    let at = cell_point(col, row);
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: at,
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        buttons: if matches!(phase, PointerPhase::Up) {
            MouseButtons::empty()
        } else {
            MouseButtons::LEFT
        },
        modifiers: ModifiersState::empty(),
        phase,
    }))
}

#[must_use]
pub fn click(col: u16, row: u16) -> [ShellEvent; 2] {
    [
        pointer(PointerPhase::Down, col, row),
        pointer(PointerPhase::Up, col, row),
    ]
}

#[must_use]
pub fn wheel(col: u16, row: u16, lines: f32) -> ShellEvent {
    scroll(cell_point(col, row), 0.0, lines, ModifiersState::empty())
}

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

#[cfg(test)]
#[path = "mouse_tests.rs"]
mod tests;

const fn bit(button: MouseButton) -> MouseButtons {
    match button {
        MouseButton::Left => MouseButtons::LEFT,
        MouseButton::Right => MouseButtons::RIGHT,
        MouseButton::Middle => MouseButtons::MIDDLE,
        MouseButton::Back => MouseButtons::BACK,
        MouseButton::Forward => MouseButtons::FORWARD,
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
