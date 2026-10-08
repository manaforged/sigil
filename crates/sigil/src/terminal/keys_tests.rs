use crossterm::event::{Event, KeyCode, KeyEvent as TerminalKeyEvent, KeyModifiers};

use super::*;

#[test]
fn shift_enter_is_a_newline_chord_when_the_terminal_omits_shift() {
    let bare = ModifiersState::empty();
    assert_eq!(
        note_enter_shift(bare, true, true),
        ModifiersState::SHIFT,
        "Enter while Shift is held inserts a newline"
    );
    assert_eq!(note_enter_shift(bare, true, false), bare);
    assert_eq!(note_enter_shift(bare, false, true), bare);
}

#[test]
fn terminal_function_keys_reach_application_shortcuts() -> Result<(), String> {
    for (number, expected) in [(1, Key::F1), (5, Key::F5), (35, Key::F35)] {
        let event = TerminalKeyEvent::new(KeyCode::F(number), KeyModifiers::SHIFT);
        let Some(ShellEvent::Key(mapped)) = map_key(&event) else {
            return Err(format!("F{number} was dropped"));
        };
        assert_eq!(mapped.key, LogicalKey::Named(expected));
        assert_eq!(mapped.modifiers, ModifiersState::SHIFT);
        assert!(mapped.text.is_none());
    }
    for number in [0, 36, u8::MAX] {
        assert!(
            map_key(&TerminalKeyEvent::new(
                KeyCode::F(number),
                KeyModifiers::NONE
            ))
            .is_none()
        );
    }
    Ok(())
}

#[test]
fn shifted_letters_map_to_uppercase() {
    let event = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('z'),
        crossterm::event::KeyModifiers::CONTROL | crossterm::event::KeyModifiers::SHIFT,
    );
    let key = map_key(&event)
        .and_then(|mapped| match mapped {
            ShellEvent::Key(key) => Some(key),
            _ => None,
        })
        .expect("shifted letter did not map");
    assert_eq!(key.key, LogicalKey::character("Z"));
    assert!(
        key.modifiers
            .contains(ModifiersState::SHIFT | ModifiersState::CONTROL)
    );
}

#[test]
fn committed_cjk_text_arrives_as_one_insert() {
    let mut events = Vec::new();
    for ch in ['日', '本', '語'] {
        if let Some(event) = map_key(&TerminalKeyEvent::new(
            KeyCode::Char(ch),
            KeyModifiers::NONE,
        )) {
            push_key_text(&mut events, event);
        }
    }
    assert_eq!(events.len(), 1, "{events:?}");
    let mut ascii = Vec::new();
    for ch in ['a', 'b'] {
        if let Some(event) = map_key(&TerminalKeyEvent::new(
            KeyCode::Char(ch),
            KeyModifiers::NONE,
        )) {
            push_key_text(&mut ascii, event);
        }
    }
    assert_eq!(ascii.len(), 2, "ASCII keys stay separate for shortcuts");
}

fn typed_keys(text: &str) -> Vec<Event> {
    text.chars()
        .map(|ch| match ch {
            '\n' => key(KeyCode::Enter, KeyModifiers::NONE),
            ch => key(KeyCode::Char(ch), KeyModifiers::NONE),
        })
        .collect()
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> Event {
    Event::Key(TerminalKeyEvent::new(code, modifiers))
}

#[test]
fn late_osc_and_apc_replies_never_become_keys() {
    let mut events = vec![key(KeyCode::Char(']'), KeyModifiers::ALT)];
    events.extend(typed_keys("11;rgb:0a0a/0a0a/0a0a"));
    events.push(key(KeyCode::Char('\\'), KeyModifiers::ALT));
    events.push(key(KeyCode::Char('q'), KeyModifiers::NONE));
    events.push(key(KeyCode::Char(']'), KeyModifiers::ALT));
    events.extend(typed_keys("11;rgb:ffff/ffff/ffff"));
    events.push(key(KeyCode::Char('g'), KeyModifiers::CONTROL));
    events.push(key(KeyCode::Char('_'), KeyModifiers::ALT));
    events.extend(typed_keys("Gi=31;OK"));
    events.push(key(KeyCode::Char('\\'), KeyModifiers::ALT));
    strip_replies(&mut events);
    assert_eq!(events, vec![key(KeyCode::Char('q'), KeyModifiers::NONE)]);
}

#[test]
fn an_unterminated_alt_bracket_stays_a_key() {
    let mut events = vec![
        key(KeyCode::Char(']'), KeyModifiers::ALT),
        key(KeyCode::Char('x'), KeyModifiers::NONE),
    ];
    let before = events.clone();
    strip_replies(&mut events);
    assert_eq!(events, before);
}

#[test]
fn an_unbracketed_multi_line_burst_arrives_as_one_paste() {
    let pasted = coalesce_paste(typed_keys("first\nsecond"));
    assert_eq!(pasted, vec![Event::Paste("first\nsecond".to_owned())]);
    let submitted = typed_keys("k\n");
    assert_eq!(
        coalesce_paste(submitted.clone()),
        submitted,
        "a key and Enter stay keys so Enter still submits"
    );
}
