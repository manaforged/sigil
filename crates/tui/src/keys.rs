use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use sigil::hal::{
    Key, KeyEvent, KeyEventInit, KeyPhase, LogicalKey, ModifiersState, PhysicalKey, ShellEvent,
};
use smol_str::SmolStr;

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

#[must_use]
pub fn map_key(event: &crossterm::event::KeyEvent) -> Option<ShellEvent> {
    let (phase, repeat) = match event.kind {
        KeyEventKind::Press => (KeyPhase::Down, false),
        KeyEventKind::Repeat => (KeyPhase::Down, true),
        KeyEventKind::Release => (KeyPhase::Up, false),
    };
    let code = shifted(event);
    let key = logical(code)?;
    let text = match code {
        KeyCode::Char(ch)
            if phase == KeyPhase::Down
                && !event
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            Some(SmolStr::from(ch.to_string()))
        }
        _ => None,
    };
    Some(ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key,
        physical: PhysicalKey::Unidentified,
        modifiers: modifiers(event),
        repeat,
        phase,
        text,
    })))
}

#[must_use]
pub fn paste(text: &str) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(text.to_string()),
        physical: PhysicalKey::Unidentified,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: Some(SmolStr::from(text)),
    }))
}

const fn shifted(event: &crossterm::event::KeyEvent) -> KeyCode {
    match event.code {
        KeyCode::Char(ch)
            if ch.is_ascii_lowercase() && event.modifiers.contains(KeyModifiers::SHIFT) =>
        {
            KeyCode::Char(ch.to_ascii_uppercase())
        }
        code => code,
    }
}

fn modifiers(event: &crossterm::event::KeyEvent) -> ModifiersState {
    let mut state = ModifiersState::empty();
    let shifted_char = matches!(event.code, KeyCode::Char(ch) if ch.is_uppercase());
    if event.modifiers.contains(KeyModifiers::SHIFT)
        || event.code == KeyCode::BackTab
        || shifted_char
    {
        state |= ModifiersState::SHIFT;
    }
    if event.modifiers.contains(KeyModifiers::CONTROL) {
        state |= ModifiersState::CONTROL;
    }
    if event.modifiers.contains(KeyModifiers::ALT) {
        state |= ModifiersState::ALT;
    }
    if event
        .modifiers
        .intersects(KeyModifiers::SUPER | KeyModifiers::HYPER | KeyModifiers::META)
    {
        state |= ModifiersState::META;
    }
    state
}

fn logical(code: KeyCode) -> Option<LogicalKey> {
    Some(match code {
        KeyCode::Enter => LogicalKey::Named(Key::Enter),
        KeyCode::Esc => LogicalKey::Named(Key::Escape),
        KeyCode::Tab | KeyCode::BackTab => LogicalKey::Named(Key::Tab),
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
        KeyCode::F(number) => LogicalKey::Named(match format!("F{number}").parse() {
            Ok(key) => key,
            Err(_unsupported) => return None,
        }),
        KeyCode::Char(ch) => LogicalKey::character(ch.to_string()),
        _ => return None,
    })
}

#[cfg(test)]
#[path = "keys_tests.rs"]
mod tests;
