//! Map terminal keys onto [`sigil_hal::ShellEvent`].

use sigil_hal::{
    Key, KeyEvent, KeyEventInit, KeyPhase, LogicalKey, ModifiersState, PhysicalKey, ShellEvent,
};

/// Build a key-down event for tests and the live host.
#[must_use]
pub fn key_down(key: impl Into<LogicalKey>) -> ShellEvent {
    let key = key.into();
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key,
        physical: PhysicalKey::Unidentified,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

/// Translate a crossterm key press into a Sigil shell event.
#[must_use]
pub fn map_key(event: &crossterm::event::KeyEvent) -> Option<ShellEvent> {
    if event.kind != crossterm::event::KeyEventKind::Press {
        return None;
    }
    let key = logical(event.code)?;
    let mut modifiers = ModifiersState::empty();
    if event.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
        modifiers |= ModifiersState::SHIFT;
    }
    if event.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
        modifiers |= ModifiersState::CONTROL;
    }
    if event.modifiers.contains(crossterm::event::KeyModifiers::ALT) {
        modifiers |= ModifiersState::ALT;
    }
    Some(ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key,
        physical: PhysicalKey::Unidentified,
        modifiers,
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    })))
}

fn logical(code: crossterm::event::KeyCode) -> Option<LogicalKey> {
    use crossterm::event::KeyCode;
    Some(match code {
        KeyCode::Enter => LogicalKey::Named(Key::Enter),
        KeyCode::Esc => LogicalKey::Named(Key::Escape),
        KeyCode::Tab => LogicalKey::Named(Key::Tab),
        KeyCode::Backspace => LogicalKey::Named(Key::Backspace),
        KeyCode::Delete => LogicalKey::Named(Key::Delete),
        KeyCode::Left => LogicalKey::Named(Key::ArrowLeft),
        KeyCode::Right => LogicalKey::Named(Key::ArrowRight),
        KeyCode::Up => LogicalKey::Named(Key::ArrowUp),
        KeyCode::Down => LogicalKey::Named(Key::ArrowDown),
        KeyCode::Home => LogicalKey::Named(Key::Home),
        KeyCode::End => LogicalKey::Named(Key::End),
        KeyCode::PageUp => LogicalKey::Named(Key::PageUp),
        KeyCode::PageDown => LogicalKey::Named(Key::PageDown),
        KeyCode::Char(ch) => LogicalKey::character(ch.to_string()),
        _ => return None,
    })
}
