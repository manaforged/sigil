use crate::events::{
    Key, KeyEvent, KeyEventInit, KeyPhase, LogicalKey, ModifiersState, PhysicalKey, ShellEvent,
};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MediaKeyCode, ModifierKeyCode};
use smol_str::SmolStr;

#[cfg(any(windows, test))]
const PASTE_BURST: usize = 8;
#[cfg(windows)]
pub(crate) const PASTE_GAP: std::time::Duration = std::time::Duration::from_millis(25);

#[must_use]
pub fn key_down(key: impl Into<LogicalKey>) -> ShellEvent {
    pressed_key(key.into(), None)
}

fn pressed_key(key: LogicalKey, text: Option<SmolStr>) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key,
        physical: PhysicalKey::Unidentified,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text,
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
    let altgr = event
        .modifiers
        .contains(KeyModifiers::CONTROL | KeyModifiers::ALT)
        && matches!(code, KeyCode::Char(ch) if !ch.is_ascii_alphanumeric());
    let text = match code {
        KeyCode::Char(ch)
            if phase == KeyPhase::Down
                && (altgr
                    || !event
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)) =>
        {
            Some(SmolStr::from(ch.to_string()))
        }
        _ => None,
    };
    let mut held = modifiers(event);
    if altgr {
        held -= ModifiersState::CONTROL | ModifiersState::ALT;
    }
    Some(ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key,
        physical: PhysicalKey::Unidentified,
        modifiers: held,
        repeat,
        phase,
        text,
    })))
}

pub(crate) fn push_key_text(events: &mut Vec<ShellEvent>, event: ShellEvent) {
    let merged = events
        .last()
        .and_then(composed_text)
        .zip(composed_text(&event))
        .map(|(head, tail)| SmolStr::from(format!("{head}{tail}")));
    match merged {
        Some(text) => {
            events.pop();
            events.push(pressed_key(LogicalKey::Character(text.clone()), Some(text)));
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
#[cfg(any(windows, test))]
pub(crate) fn burst_paste(burst: &[Event], follows_paste: bool) -> Option<String> {
    let mut text = String::new();
    let mut lines = 0_usize;
    let mut inner = false;
    for event in burst {
        let Event::Key(key) = event else {
            return None;
        };
        if key.kind == KeyEventKind::Release {
            continue;
        }
        if !(key.modifiers - KeyModifiers::SHIFT).is_empty() {
            return None;
        }
        match key.code {
            KeyCode::Char(ch) => text.push(ch),
            KeyCode::Enter => {
                text.push('\n');
                lines += 1;
            }
            KeyCode::Tab => text.push('\t'),
            _ => return None,
        }
        inner |= lines > 0 && key.code != KeyCode::Enter;
    }
    let long = lines > 0 && text.chars().count() >= PASTE_BURST;
    (!text.is_empty() && (follows_paste || inner || long)).then_some(text)
}

pub fn paste(text: &str) -> ShellEvent {
    ShellEvent::Paste(text.to_string())
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
    let mut state = held(event.modifiers);
    let shifted_char = matches!(event.code, KeyCode::Char(ch) if ch.is_uppercase());
    if event.code == KeyCode::BackTab || shifted_char {
        state |= ModifiersState::SHIFT;
    }
    state
}

pub(crate) fn held(from: KeyModifiers) -> ModifiersState {
    let mut state = ModifiersState::empty();
    state.set(ModifiersState::SHIFT, from.contains(KeyModifiers::SHIFT));
    state.set(
        ModifiersState::CONTROL,
        from.contains(KeyModifiers::CONTROL),
    );
    state.set(ModifiersState::ALT, from.contains(KeyModifiers::ALT));
    let meta = KeyModifiers::SUPER | KeyModifiers::HYPER | KeyModifiers::META;
    state.set(ModifiersState::META, from.intersects(meta));
    state
}

fn logical(code: KeyCode) -> Option<LogicalKey> {
    let key = match code {
        KeyCode::Char(ch) => return Some(LogicalKey::character(ch.to_string())),
        KeyCode::Enter => Key::Enter,
        KeyCode::Esc => Key::Escape,
        KeyCode::Tab | KeyCode::BackTab => Key::Tab,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Delete => Key::Delete,
        KeyCode::Left => Key::ArrowLeft,
        KeyCode::Right => Key::ArrowRight,
        KeyCode::Up => Key::ArrowUp,
        KeyCode::Down => Key::ArrowDown,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::F(number) => format!("F{number}").parse().ok()?,
        KeyCode::Insert => Key::Insert,
        KeyCode::CapsLock => Key::CapsLock,
        KeyCode::ScrollLock => Key::ScrollLock,
        KeyCode::NumLock => Key::NumLock,
        KeyCode::PrintScreen => Key::PrintScreen,
        KeyCode::Pause => Key::Pause,
        KeyCode::Menu => Key::ContextMenu,
        KeyCode::Media(media) => media_key(media)?,
        KeyCode::Modifier(modifier) => modifier_key(modifier),
        _ => return None,
    };
    Some(LogicalKey::Named(key))
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
