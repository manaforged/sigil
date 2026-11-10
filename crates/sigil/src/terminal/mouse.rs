use crate::events::{
    Event, ModifiersState, MouseButton, MouseButtons, PointerPhase, ScrollDelta, ScrollPhase,
};
use crate::geometry::Point;
use crossterm::event::{MouseButton as TermButton, MouseEvent, MouseEventKind};

const NOTCH: f32 = 3.0;

pub(crate) const MOUSE_OFF: &str = "\x1b[?1006l\x1b[?1003l\x1b[?1002l\x1b[?1000l";

/// Terminal mouse reports, all in SGR form (1006).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum MouseTracking {
    /// The off mouse tracking.
    Off,
    /// The clicks mouse tracking.
    Clicks,
    /// The drags mouse tracking.
    #[default]
    Drags,
    /// The motion mouse tracking.
    Motion,
}

impl MouseTracking {
    pub(crate) const fn sequence(self) -> &'static str {
        match self {
            Self::Off => "",
            Self::Clicks => "\x1b[?1000h\x1b[?1006h",
            Self::Drags => "\x1b[?1000h\x1b[?1002h\x1b[?1006h",
            Self::Motion => "\x1b[?1000h\x1b[?1002h\x1b[?1003h\x1b[?1006h",
        }
    }
}

/// Returns the layout point of a cell.
#[cfg(any(test, feature = "kittest"))]
#[must_use]
pub fn cell_point(col: u16, row: u16) -> Point {
    centre(col, row, crate::Host::CELL)
}

fn centre(col: u16, row: u16, cell: f32) -> Point {
    Point::new((f32::from(col) + 0.5) * cell, (f32::from(row) + 0.5) * cell)
}

/// Converts a crossterm mouse event to a Sigil event.
#[must_use]
pub fn map_mouse(event: MouseEvent, cell: f32, last: &mut Point) -> Event {
    let at = centre(event.column, event.row, cell);
    let delta = Point::new(at.x - last.x, at.y - last.y);
    *last = at;
    let modifiers = crate::terminal::keys::held(event.modifiers);
    let (phase, button) = match event.kind {
        MouseEventKind::Down(button) => (PointerPhase::Down, Some(button)),
        MouseEventKind::Up(button) => (PointerPhase::Up, Some(button)),
        MouseEventKind::Drag(button) => (PointerPhase::Move, Some(button)),
        MouseEventKind::Moved => (PointerPhase::Move, None),
        MouseEventKind::ScrollUp => return scroll(at, 0.0, NOTCH, modifiers),
        MouseEventKind::ScrollDown => return scroll(at, 0.0, -NOTCH, modifiers),
        MouseEventKind::ScrollLeft => return scroll(at, NOTCH, 0.0, modifiers),
        MouseEventKind::ScrollRight => return scroll(at, -NOTCH, 0.0, modifiers),
    };
    let button = button.map(press);
    let held = match phase {
        PointerPhase::Up => MouseButtons::empty(),
        _ => button.map_or_else(MouseButtons::empty, bit),
    };
    Event::pointer(phase, at)
        .delta(delta)
        .button(button)
        .buttons(held)
        .modifiers(modifiers)
}

const fn scroll(at: Point, x: f32, y: f32, modifiers: ModifiersState) -> Event {
    Event::scroll(at, ScrollDelta::Lines(Point::new(x, y)), ScrollPhase::Moved).modifiers(modifiers)
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
        MouseButton::Other(_) => MouseButtons::EMPTY,
    }
}
