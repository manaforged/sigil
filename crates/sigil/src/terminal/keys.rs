use crate::events::{
    Key, KeyEvent, KeyEventInit, KeyPhase, LogicalKey, ModifiersState, PhysicalKey, ShellEvent,
};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MediaKeyCode, ModifierKeyCode};
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

pub(crate) fn push_key_text(events: &mut Vec<ShellEvent>, event: ShellEvent) {
    let merged = match (events.last(), &event) {
        (Some(last), next) => composed_text(last)
            .zip(composed_text(next))
            .map(|(head, tail)| text_key(&format!("{head}{tail}"))),
        (None, _) => None,
    };
    match merged {
        Some(merged) => {
            events.pop();
            events.push(merged);
        }
        None => events.push(event),
    }
}

fn composed_text(event: &ShellEvent) -> Option<&str> {
    let ShellEvent::Key(key) = event else {
        return None;
    };
    let text = key.text.as_deref()?;
    let plain = key.phase == KeyPhase::Down
        && !key.repeat
        && (key.modifiers - ModifiersState::SHIFT).is_empty();
    (plain && !text.is_empty() && !text.is_ascii() && !text.chars().any(char::is_control))
        .then_some(text)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Reply {
    Osc,
    Apc,
}

pub(crate) fn strip_replies(events: &mut Vec<Event>) {
    let mut kept = Vec::with_capacity(events.len());
    let mut open = Vec::new();
    let mut reply = None;
    for event in events.drain(..) {
        match reply {
            None => {
                reply = opener(&event);
                if reply.is_some() {
                    open.push(event);
                } else {
                    kept.push(event);
                }
            }
            Some(kind) if closes(kind, &event) => {
                open.clear();
                reply = None;
            }
            Some(_) => open.push(event),
        }
    }
    kept.append(&mut open);
    *events = kept;
}

fn opener(event: &Event) -> Option<Reply> {
    let (code, modifiers) = pressed(event)?;
    if modifiers != KeyModifiers::ALT {
        return None;
    }
    match code {
        KeyCode::Char(']') => Some(Reply::Osc),
        KeyCode::Char('_') => Some(Reply::Apc),
        _ => None,
    }
}

fn closes(kind: Reply, event: &Event) -> bool {
    match pressed(event) {
        Some((KeyCode::Char('\\'), modifiers)) => modifiers == KeyModifiers::ALT,
        Some((KeyCode::Char('g'), modifiers)) => {
            kind == Reply::Osc && modifiers == KeyModifiers::CONTROL
        }
        _ => false,
    }
}

fn pressed(event: &Event) -> Option<(KeyCode, KeyModifiers)> {
    match event {
        Event::Key(key) if key.kind != KeyEventKind::Release => Some((key.code, key.modifiers)),
        _ => None,
    }
}

#[must_use]
pub fn paste(text: &str) -> ShellEvent {
    ShellEvent::Paste(text.to_string())
}

fn text_key(text: &str) -> ShellEvent {
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
        other => LogicalKey::Named(extra(other)?),
    })
}

const fn extra(code: KeyCode) -> Option<Key> {
    Some(match code {
        KeyCode::Insert => Key::Insert,
        KeyCode::CapsLock => Key::CapsLock,
        KeyCode::ScrollLock => Key::ScrollLock,
        KeyCode::NumLock => Key::NumLock,
        KeyCode::PrintScreen => Key::PrintScreen,
        KeyCode::Pause => Key::Pause,
        KeyCode::Menu => Key::ContextMenu,
        KeyCode::Media(media) => return media_key(media),
        KeyCode::Modifier(modifier) => modifier_key(modifier),
        _ => return None,
    })
}

const fn media_key(media: MediaKeyCode) -> Option<Key> {
    Some(match media {
        MediaKeyCode::Play => Key::MediaPlay,
        MediaKeyCode::Pause => Key::MediaPause,
        MediaKeyCode::PlayPause => Key::MediaPlayPause,
        MediaKeyCode::Stop => Key::MediaStop,
        MediaKeyCode::TrackNext => Key::MediaTrackNext,
        MediaKeyCode::TrackPrevious => Key::MediaTrackPrevious,
        MediaKeyCode::FastForward => Key::MediaFastForward,
        MediaKeyCode::Rewind => Key::MediaRewind,
        MediaKeyCode::Record => Key::MediaRecord,
        MediaKeyCode::LowerVolume => Key::AudioVolumeDown,
        MediaKeyCode::RaiseVolume => Key::AudioVolumeUp,
        MediaKeyCode::MuteVolume => Key::AudioVolumeMute,
        MediaKeyCode::Reverse => return None,
    })
}

const fn modifier_key(modifier: ModifierKeyCode) -> Key {
    match modifier {
        ModifierKeyCode::LeftShift
        | ModifierKeyCode::RightShift
        | ModifierKeyCode::IsoLevel5Shift => Key::Shift,
        ModifierKeyCode::LeftControl | ModifierKeyCode::RightControl => Key::Control,
        ModifierKeyCode::LeftAlt | ModifierKeyCode::RightAlt => Key::Alt,
        ModifierKeyCode::LeftSuper
        | ModifierKeyCode::RightSuper
        | ModifierKeyCode::LeftHyper
        | ModifierKeyCode::RightHyper
        | ModifierKeyCode::LeftMeta
        | ModifierKeyCode::RightMeta => Key::Meta,
        ModifierKeyCode::IsoLevel3Shift => Key::AltGraph,
    }
}

#[cfg(test)]
#[path = "keys_tests.rs"]
mod tests;
