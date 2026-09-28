#![allow(dead_code)]

use sigil::events::prelude::*;
use sigil::hal::{KeyEventInit, Point, PointerEventInit, ScrollEventInit};

pub(crate) const fn pointer_down(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        buttons: MouseButtons::LEFT,
        modifiers: ModifiersState::empty(),
        phase: PointerPhase::Down,
    }))
}

pub(crate) const fn pointer_move(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: None,
        buttons: MouseButtons::LEFT,
        modifiers: ModifiersState::empty(),
        phase: PointerPhase::Move,
    }))
}

pub(crate) const fn pointer_up(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        buttons: MouseButtons::empty(),
        modifiers: ModifiersState::empty(),
        phase: PointerPhase::Up,
    }))
}

pub(crate) const fn pointer_move_event(x: f32, y: f32) -> ShellEvent {
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

pub(crate) fn key_down(key: Key, physical: PhysicalKey) -> ShellEvent {
    key_event(key, physical, KeyPhase::Down)
}

pub(crate) fn type_char(ch: &str, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(ch),
        physical,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: Some(ch.to_string().into()),
    }))
}

pub(crate) fn tab(shift: bool) -> ShellEvent {
    let mut modifiers = ModifiersState::empty();
    modifiers.set(ModifiersState::SHIFT, shift);
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: Key::Tab.into(),
        physical: PhysicalKey::Tab,
        modifiers,
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

pub(crate) fn ctrl_s() -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character("s"),
        physical: PhysicalKey::KeyS,
        modifiers: ModifiersState::CONTROL,
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

pub(crate) fn key_event(key: Key, physical: PhysicalKey, phase: KeyPhase) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: key.into(),
        physical,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase,
        text: None,
    }))
}

pub(crate) const fn window_blur() -> ShellEvent {
    ShellEvent::Window(sigil::hal::WindowEvent::Focused(false))
}

pub(crate) const fn scroll_lines(x: f32, y: f32, dy: f32) -> ShellEvent {
    ShellEvent::Scroll(ScrollEvent::new(ScrollEventInit {
        position: Point::new(x, y),
        delta: ScrollDelta::Lines(Point::new(0.0, dy)),
        modifiers: ModifiersState::empty(),
        phase: ScrollPhase::Moved,
    }))
}

pub(crate) const fn ime_event(event: ImeEvent) -> ShellEvent {
    ShellEvent::Ime(event)
}
