use crate::events::{Event, Key, KeyPhase, Modifiers, NamedKey};
use crossterm::event::{
    Event as TermEvent, KeyCode, KeyEventKind, KeyModifiers, MediaKeyCode, ModifierKeyCode,
};

#[cfg(any(windows, test))]
const PASTE_BURST: usize = 8;
#[cfg(windows)]
pub(crate) const PASTE_GAP: std::time::Duration = std::time::Duration::from_millis(25);

fn pressed_key(key: Key, text: Option<String>) -> Event {
    Event::key(key, KeyPhase::Down).text(text)
}

kittest_pub! {
    /// Translate a crossterm key event to an `Event`.
    #[doc(hidden)]
    #[must_use]
    pub fn map_key(event: &crossterm::event::KeyEvent) -> Option<Event> {
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
                Some(ch.to_string())
            }
            _ => None,
        };
        let mut held = modifiers(event);
        if altgr {
            held -= Modifiers::CTRL | Modifiers::ALT;
        }
        Some(
            Event::key(key, phase)
                .modifiers(held)
                .repeat(repeat)
                .text(text),
        )
    }
}

pub(crate) fn push_key_text(events: &mut Vec<Event>, event: Event) {
    let merged = events
        .last()
        .and_then(composed_text)
        .zip(composed_text(&event))
        .map(|(head, tail)| format!("{head}{tail}"));
    match merged {
        Some(text) => {
            events.pop();
            events.push(pressed_key(Key::Character(text.clone()), Some(text)));
        }
        None => events.push(event),
    }
}

fn composed_text(event: &Event) -> Option<&str> {
    let Event::Key(key) = event else {
        return None;
    };
    let text = key.text.as_deref()?;
    let plain =
        key.phase == KeyPhase::Down && !key.repeat && (key.modifiers - Modifiers::SHIFT).is_empty();
    (plain && !text.is_empty() && !text.is_ascii() && !text.chars().any(char::is_control))
        .then_some(text)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Reply {
    Osc,
    Apc,
    Dcs,
}

pub(crate) fn strip_replies(events: &mut Vec<TermEvent>) {
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

fn opener(event: &TermEvent) -> Option<Reply> {
    let (code, modifiers) = pressed(event)?;
    match code {
        KeyCode::Char(']') if modifiers == KeyModifiers::ALT => Some(Reply::Osc),
        KeyCode::Char('_') if modifiers == KeyModifiers::ALT => Some(Reply::Apc),
        KeyCode::Char('P') if modifiers == KeyModifiers::ALT | KeyModifiers::SHIFT => {
            Some(Reply::Dcs)
        }
        _ => None,
    }
}

fn closes(kind: Reply, event: &TermEvent) -> bool {
    match pressed(event) {
        Some((KeyCode::Char('\\'), modifiers)) => modifiers == KeyModifiers::ALT,
        Some((KeyCode::Char('g'), modifiers)) => {
            kind == Reply::Osc && modifiers == KeyModifiers::CONTROL
        }
        _ => false,
    }
}

fn pressed(event: &TermEvent) -> Option<(KeyCode, KeyModifiers)> {
    match event {
        TermEvent::Key(key) if key.kind != KeyEventKind::Release => Some((key.code, key.modifiers)),
        _ => None,
    }
}

#[must_use]
#[cfg(any(windows, test))]
pub(crate) fn burst_paste(burst: &[TermEvent], follows_paste: bool) -> Option<String> {
    let mut text = String::new();
    let mut lines = 0_usize;
    let mut inner = false;
    for event in burst {
        let TermEvent::Key(key) = event else {
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

fn modifiers(event: &crossterm::event::KeyEvent) -> Modifiers {
    let mut state = held(event.modifiers);
    let shifted_char = matches!(event.code, KeyCode::Char(ch) if ch.is_uppercase());
    if event.code == KeyCode::BackTab || shifted_char {
        state |= Modifiers::SHIFT;
    }
    state
}

pub(crate) fn held(from: KeyModifiers) -> Modifiers {
    let mut state = Modifiers::empty();
    state.set(Modifiers::SHIFT, from.contains(KeyModifiers::SHIFT));
    state.set(Modifiers::CTRL, from.contains(KeyModifiers::CONTROL));
    state.set(Modifiers::ALT, from.contains(KeyModifiers::ALT));
    let meta = KeyModifiers::SUPER | KeyModifiers::HYPER | KeyModifiers::META;
    state.set(Modifiers::META, from.intersects(meta));
    state
}

fn logical(code: KeyCode) -> Option<Key> {
    let key = match code {
        KeyCode::Char(ch) => return Some(Key::character(ch.to_string())),
        KeyCode::Enter => NamedKey::Enter,
        KeyCode::Esc => NamedKey::Escape,
        KeyCode::Tab | KeyCode::BackTab => NamedKey::Tab,
        KeyCode::Backspace => NamedKey::Backspace,
        KeyCode::Delete => NamedKey::Delete,
        KeyCode::Left => NamedKey::ArrowLeft,
        KeyCode::Right => NamedKey::ArrowRight,
        KeyCode::Up => NamedKey::ArrowUp,
        KeyCode::Down => NamedKey::ArrowDown,
        KeyCode::Home => NamedKey::Home,
        KeyCode::End => NamedKey::End,
        KeyCode::PageUp => NamedKey::PageUp,
        KeyCode::PageDown => NamedKey::PageDown,
        KeyCode::F(number) => format!("F{number}").parse().ok()?,
        KeyCode::Insert => NamedKey::Insert,
        KeyCode::CapsLock => NamedKey::CapsLock,
        KeyCode::ScrollLock => NamedKey::ScrollLock,
        KeyCode::NumLock => NamedKey::NumLock,
        KeyCode::PrintScreen => NamedKey::PrintScreen,
        KeyCode::Pause => NamedKey::Pause,
        KeyCode::Menu => NamedKey::ContextMenu,
        KeyCode::Media(media) => media_key(media)?,
        KeyCode::Modifier(modifier) => modifier_key(modifier),
        _ => return None,
    };
    Some(Key::Named(key))
}

const fn media_key(media: MediaKeyCode) -> Option<NamedKey> {
    Some(match media {
        MediaKeyCode::Play => NamedKey::MediaPlay,
        MediaKeyCode::Pause => NamedKey::MediaPause,
        MediaKeyCode::PlayPause => NamedKey::MediaPlayPause,
        MediaKeyCode::Stop => NamedKey::MediaStop,
        MediaKeyCode::TrackNext => NamedKey::MediaTrackNext,
        MediaKeyCode::TrackPrevious => NamedKey::MediaTrackPrevious,
        MediaKeyCode::FastForward => NamedKey::MediaFastForward,
        MediaKeyCode::Rewind => NamedKey::MediaRewind,
        MediaKeyCode::Record => NamedKey::MediaRecord,
        MediaKeyCode::LowerVolume => NamedKey::AudioVolumeDown,
        MediaKeyCode::RaiseVolume => NamedKey::AudioVolumeUp,
        MediaKeyCode::MuteVolume => NamedKey::AudioVolumeMute,
        MediaKeyCode::Reverse => return None,
    })
}

const fn modifier_key(modifier: ModifierKeyCode) -> NamedKey {
    match modifier {
        ModifierKeyCode::LeftShift
        | ModifierKeyCode::RightShift
        | ModifierKeyCode::IsoLevel5Shift => NamedKey::Shift,
        ModifierKeyCode::LeftControl | ModifierKeyCode::RightControl => NamedKey::Control,
        ModifierKeyCode::LeftAlt | ModifierKeyCode::RightAlt => NamedKey::Alt,
        ModifierKeyCode::LeftSuper
        | ModifierKeyCode::RightSuper
        | ModifierKeyCode::LeftHyper
        | ModifierKeyCode::RightHyper
        | ModifierKeyCode::LeftMeta
        | ModifierKeyCode::RightMeta => NamedKey::Meta,
        ModifierKeyCode::IsoLevel3Shift => NamedKey::AltGraph,
    }
}

#[cfg(test)]
#[path = "keys_tests.rs"]
mod tests;

#[cfg(fuzzing)]
pub(crate) fn fuzz_strip(input: &[u8]) {
    let events: Vec<TermEvent> = input.iter().map(|byte| fuzz_event(*byte)).collect();
    let mut once = events.clone();
    strip_replies(&mut once);
    let mut rest = events.iter();
    assert!(
        once.iter().all(|kept| rest.any(|event| event == kept)),
        "the reply filter invented or reordered input"
    );
    if events.iter().all(|event| opener(event).is_none()) {
        assert_eq!(once, events, "input without a reply opener changed");
    }
    let mut twice = once.clone();
    strip_replies(&mut twice);
    assert_eq!(twice, once, "the reply filter is not idempotent");
}

#[cfg(fuzzing)]
fn fuzz_event(byte: u8) -> TermEvent {
    let (code, modifiers) = match byte % 8 {
        0 => (KeyCode::Char(']'), KeyModifiers::ALT),
        1 => (KeyCode::Char('_'), KeyModifiers::ALT),
        2 => (KeyCode::Char('P'), KeyModifiers::ALT | KeyModifiers::SHIFT),
        3 => (KeyCode::Char('\\'), KeyModifiers::ALT),
        4 => (KeyCode::Char('g'), KeyModifiers::CONTROL),
        _ => (
            KeyCode::Char(char::from(b'a' + byte / 8 % 26)),
            KeyModifiers::NONE,
        ),
    };
    TermEvent::Key(crossterm::event::KeyEvent::new(code, modifiers))
}
