use crossterm::event::{Event, KeyCode, KeyEvent as TerminalKeyEvent, KeyModifiers};

use super::*;

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
fn altgr_characters_type_as_text() -> Result<(), String> {
    for ch in ['@', '{', '€', 'ą'] {
        let event =
            TerminalKeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL | KeyModifiers::ALT);
        let Some(ShellEvent::Key(mapped)) = map_key(&event) else {
            return Err(format!("{ch} was dropped"));
        };
        assert_eq!(mapped.text.as_deref(), Some(ch.to_string().as_str()));
        assert!(
            !mapped
                .modifiers
                .intersects(ModifiersState::CONTROL | ModifiersState::ALT)
        );
    }
    let shortcut = TerminalKeyEvent::new(
        KeyCode::Char('k'),
        KeyModifiers::CONTROL | KeyModifiers::ALT,
    );
    let Some(ShellEvent::Key(mapped)) = map_key(&shortcut) else {
        return Err("ctrl+alt+k was dropped".to_string());
    };
    assert!(mapped.text.is_none());
    Ok(())
}

#[test]
fn a_burst_of_typed_lines_reads_as_one_paste() {
    let mut burst = typed_keys("first line");
    burst.push(key(KeyCode::Enter, KeyModifiers::NONE));
    burst.extend(typed_keys("second"));
    assert_eq!(
        burst_paste(&burst, false).as_deref(),
        Some("first line\nsecond")
    );
    assert_eq!(
        burst_paste(&typed_keys("no newline here at all"), false),
        None
    );
    let mut short = typed_keys("hi");
    short.push(key(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(burst_paste(&short, false), None);
    let mut chord = typed_keys("first line");
    chord.push(key(KeyCode::Enter, KeyModifiers::CONTROL));
    assert_eq!(burst_paste(&chord, false), None);
}

#[test]
fn a_short_paste_and_its_last_fragment_stay_one_paste() {
    let mut two = typed_keys("a");
    two.push(key(KeyCode::Enter, KeyModifiers::NONE));
    two.extend(typed_keys("b"));
    assert_eq!(burst_paste(&two, false).as_deref(), Some("a\nb"));
    let mut tail = typed_keys("ok");
    tail.push(key(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(burst_paste(&tail, false), None);
    assert_eq!(burst_paste(&tail, true).as_deref(), Some("ok\n"));
}
