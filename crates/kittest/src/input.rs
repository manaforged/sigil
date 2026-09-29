use sigil::events::{
    ImeEvent, Key, KeyEvent, KeyEventInit, KeyPhase, LogicalKey, ModifiersState, MouseButton,
    MouseButtons, PhysicalKey, PointerEvent, PointerEventInit, PointerKind, PointerPhase,
    ScrollDelta, ScrollEvent, ScrollEventInit, ScrollPhase, ShellEvent,
};
use sigil::geometry::Point;

const fn pointer(x: f32, y: f32, phase: PointerPhase, held: MouseButtons) -> ShellEvent {
    let button = match phase {
        PointerPhase::Down | PointerPhase::Up => Some(MouseButton::Left),
        _ => None,
    };
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button,
        buttons: held,
        modifiers: ModifiersState::empty(),
        phase,
    }))
}

#[must_use]
pub const fn pointer_move(x: f32, y: f32) -> ShellEvent {
    pointer(x, y, PointerPhase::Move, MouseButtons::empty())
}

#[must_use]
pub const fn pointer_drag(x: f32, y: f32) -> ShellEvent {
    pointer(x, y, PointerPhase::Move, MouseButtons::LEFT)
}

#[must_use]
pub const fn pointer_down(x: f32, y: f32) -> ShellEvent {
    pointer(x, y, PointerPhase::Down, MouseButtons::LEFT)
}

#[must_use]
pub const fn pointer_up(x: f32, y: f32) -> ShellEvent {
    pointer(x, y, PointerPhase::Up, MouseButtons::empty())
}

#[must_use]
pub const fn click(x: f32, y: f32) -> [ShellEvent; 3] {
    [pointer_move(x, y), pointer_down(x, y), pointer_up(x, y)]
}

#[must_use]
pub fn key_event(key: impl Into<LogicalKey>, physical: PhysicalKey, phase: KeyPhase) -> ShellEvent {
    chord_event(key.into(), physical, ModifiersState::empty(), phase)
}

#[must_use]
pub fn key_down(key: impl Into<LogicalKey>, physical: PhysicalKey) -> ShellEvent {
    key_event(key, physical, KeyPhase::Down)
}

#[must_use]
pub fn chord(
    key: impl Into<LogicalKey>,
    physical: PhysicalKey,
    modifiers: ModifiersState,
) -> ShellEvent {
    chord_event(key.into(), physical, modifiers, KeyPhase::Down)
}

fn chord_event(
    key: LogicalKey,
    physical: PhysicalKey,
    modifiers: ModifiersState,
    phase: KeyPhase,
) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key,
        physical,
        modifiers,
        repeat: false,
        phase,
        text: None,
    }))
}

#[must_use]
pub fn press(key: impl Into<LogicalKey>, modifiers: ModifiersState) -> [ShellEvent; 2] {
    let key = key.into();
    let unknown = PhysicalKey::Unidentified;
    [
        chord_event(key.clone(), unknown, modifiers, KeyPhase::Down),
        chord_event(key, unknown, modifiers, KeyPhase::Up),
    ]
}

#[must_use]
pub fn type_char(ch: &str, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(ch),
        physical,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: Some(ch.into()),
    }))
}

#[must_use]
pub fn type_text(text: &str) -> Vec<ShellEvent> {
    text.chars()
        .flat_map(|ch| {
            let ch = ch.to_string();
            let unknown = PhysicalKey::Unidentified;
            [
                type_char(&ch, unknown),
                key_event(LogicalKey::character(ch.as_str()), unknown, KeyPhase::Up),
            ]
        })
        .collect()
}

#[must_use]
pub fn tab(shift: bool) -> ShellEvent {
    let mut modifiers = ModifiersState::empty();
    modifiers.set(ModifiersState::SHIFT, shift);
    chord(Key::Tab, PhysicalKey::Tab, modifiers)
}

#[must_use]
pub const fn window_blur() -> ShellEvent {
    ShellEvent::Window(sigil::events::WindowEvent::Focused(false))
}

#[must_use]
pub const fn scroll_lines(x: f32, y: f32, dy: f32) -> ShellEvent {
    ShellEvent::Scroll(ScrollEvent::new(ScrollEventInit {
        position: Point::new(x, y),
        delta: ScrollDelta::Lines(Point::new(0.0, dy)),
        modifiers: ModifiersState::empty(),
        phase: ScrollPhase::Moved,
    }))
}

#[must_use]
pub const fn ime_event(event: ImeEvent) -> ShellEvent {
    ShellEvent::Ime(event)
}
