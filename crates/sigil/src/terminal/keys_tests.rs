use crossterm::event::{Event, KeyCode, KeyEvent as TerminalKeyEvent, KeyModifiers};

use super::*;

fn mapped(code: KeyCode, modifiers: KeyModifiers) -> Option<KeyEvent> {
    match map_key(&TerminalKeyEvent::new(code, modifiers))? {
        ShellEvent::Key(key) => Some(key),
        _ => None,
    }
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
fn terminal_function_keys_reach_application_shortcuts() {
    for (number, expected) in [(1, Key::F1), (5, Key::F5), (35, Key::F35)] {
        let key = mapped(KeyCode::F(number), KeyModifiers::SHIFT).expect("function key dropped");
        assert_eq!(key.key, LogicalKey::Named(expected));
        assert_eq!(key.modifiers, ModifiersState::SHIFT);
        assert!(key.text.is_none());
    }
    for number in [0, 36, u8::MAX] {
        assert!(mapped(KeyCode::F(number), KeyModifiers::NONE).is_none());
    }
}

#[test]
fn shifted_letters_map_to_uppercase() {
    let key = mapped(
        KeyCode::Char('z'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    )
    .expect("shifted letter did not map");
    assert_eq!(key.key, LogicalKey::character("Z"));
    assert!(
        key.modifiers
            .contains(ModifiersState::SHIFT | ModifiersState::CONTROL)
    );
}

#[test]
fn committed_cjk_text_arrives_as_one_insert() {
    let pushed = |chars: &[char]| {
        let mut events = Vec::new();
        for &ch in chars {
            if let Some(event) = map_key(&TerminalKeyEvent::new(
                KeyCode::Char(ch),
                KeyModifiers::NONE,
            )) {
                push_key_text(&mut events, event);
            }
        }
        events
    };
    let events = pushed(&['日', '本', '語']);
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(
        pushed(&['a', 'b']).len(),
        2,
        "ASCII keys stay separate for shortcuts"
    );
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
fn altgr_characters_type_as_text() {
    let altgr = KeyModifiers::CONTROL | KeyModifiers::ALT;
    for ch in ['@', '{', '€', 'ą'] {
        let key = mapped(KeyCode::Char(ch), altgr).expect("altgr character dropped");
        assert_eq!(key.text.as_deref(), Some(ch.to_string().as_str()));
        assert!(
            !key.modifiers
                .intersects(ModifiersState::CONTROL | ModifiersState::ALT)
        );
    }
    let shortcut = mapped(KeyCode::Char('k'), altgr).expect("ctrl+alt+k was dropped");
    assert!(shortcut.text.is_none());
}

#[test]
fn a_burst_of_typed_lines_reads_as_one_paste() {
    assert_eq!(
        burst_paste(&typed_keys("first line\nsecond"), false).as_deref(),
        Some("first line\nsecond")
    );
    assert_eq!(
        burst_paste(&typed_keys("no newline here at all"), false),
        None
    );
    assert_eq!(burst_paste(&typed_keys("hi\n"), false), None);
    let mut chord = typed_keys("first line");
    chord.push(key(KeyCode::Enter, KeyModifiers::CONTROL));
    assert_eq!(burst_paste(&chord, false), None);
    assert_eq!(
        burst_paste(&typed_keys("a\nb"), false).as_deref(),
        Some("a\nb")
    );
    let tail = typed_keys("ok\n");
    assert_eq!(burst_paste(&tail, false), None);
    assert_eq!(burst_paste(&tail, true).as_deref(), Some("ok\n"));
}

#[test]
fn a_late_version_reply_is_not_typed() {
    let mut events = vec![key(KeyCode::Char('a'), KeyModifiers::NONE)];
    events.push(key(
        KeyCode::Char('P'),
        KeyModifiers::ALT | KeyModifiers::SHIFT,
    ));
    events.extend(typed_keys(">|xterm(390)"));
    events.push(key(KeyCode::Char('\\'), KeyModifiers::ALT));
    events.push(key(KeyCode::Char('b'), KeyModifiers::NONE));
    strip_replies(&mut events);
    assert_eq!(
        events,
        vec![
            key(KeyCode::Char('a'), KeyModifiers::NONE),
            key(KeyCode::Char('b'), KeyModifiers::NONE)
        ]
    );
}

#[test]
fn plain_bracket_underscore_and_capital_p_are_typed_not_held() {
    for opener in [
        key(KeyCode::Char(']'), KeyModifiers::NONE),
        key(KeyCode::Char('_'), KeyModifiers::NONE),
        key(KeyCode::Char('P'), KeyModifiers::SHIFT),
        key(KeyCode::Char('P'), KeyModifiers::ALT),
    ] {
        let mut events = vec![opener.clone()];
        events.extend(typed_keys("ab"));
        events.push(key(KeyCode::Char('\\'), KeyModifiers::ALT));
        let before = events.clone();
        strip_replies(&mut events);
        assert_eq!(events, before, "{opener:?} swallowed typed keys");
    }
}
