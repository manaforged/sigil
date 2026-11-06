use crate::kernel::Cx;
use crate::reactive::{Owner, signal};

use super::terminal;

#[test]
fn terminal_builds() {
    Owner::scope("terminal-builds", || {
        let lines = signal(vec![String::from("$ ")]);
        let mut cx = Cx::new_root();
        let id = cx.add(terminal(lines, |_| {}));
        let node = cx.get(id).expect("terminal root");
        assert!(!node.children().is_empty(), "the terminal shows its lines");
    });
}

#[test]
fn keys_reach_the_shell_as_terminal_bytes() {
    use crate::events::{Key, KeyEvent, LogicalKey, ModifiersState};
    let pressed = |key: LogicalKey, modifiers: ModifiersState| {
        let mut event = KeyEvent::new(key, crate::events::KeyPhase::Down);
        event.modifiers = modifiers;
        super::key_payload(&event, false)
    };
    let none = ModifiersState::empty();
    assert_eq!(
        pressed(LogicalKey::Named(Key::ArrowUp), none).as_deref(),
        Some("\u{1b}[A")
    );
    assert_eq!(
        pressed(LogicalKey::Named(Key::Enter), none).as_deref(),
        Some("\r")
    );
    assert_eq!(
        pressed(LogicalKey::Named(Key::Escape), none).as_deref(),
        Some("\u{1b}")
    );
    assert_eq!(
        pressed(LogicalKey::Character("c".into()), ModifiersState::CONTROL).as_deref(),
        Some("\u{3}")
    );
}

#[test]
fn only_the_terminal_itself_takes_focus() {
    Owner::scope("terminal-focus", || {
        let lines = signal(vec![String::from("$ ")]);
        let mut cx = Cx::new_root();
        let root = cx.add(terminal(lines, |_| {}));
        let mut stack = cx.get(root).expect("terminal root").children().to_vec();
        while let Some(id) = stack.pop() {
            let node = cx.get(id).expect("node");
            assert!(
                !node.a11y.focusable,
                "a node inside the terminal takes focus away from it"
            );
            stack.extend(node.children().iter().copied());
        }
        assert!(cx.get(root).expect("terminal root").a11y.focusable);
    });
}

fn press(key: crate::events::LogicalKey, modifiers: crate::events::ModifiersState, application: bool) -> Option<String> {
    let mut event = crate::events::KeyEvent::new(key, crate::events::KeyPhase::Down);
    event.modifiers = modifiers;
    super::key_payload(&event, application)
}

#[test]
fn a_full_screen_program_gets_the_keys_it_expects() {
    use crate::events::{Key, LogicalKey, ModifiersState};
    let none = ModifiersState::empty();
    let named = LogicalKey::Named;
    let wanted = [
        (named(Key::ArrowUp), none, true, "\u{1b}OA"),
        (named(Key::ArrowUp), ModifiersState::CONTROL, true, "\u{1b}[1;5A"),
        (named(Key::Tab), ModifiersState::SHIFT, false, "\u{1b}[Z"),
        (named(Key::F1), none, false, "\u{1b}OP"),
        (named(Key::F5), none, false, "\u{1b}[15~"),
        (named(Key::PageUp), none, false, "\u{1b}[5~"),
        (named(Key::Enter), ModifiersState::SHIFT, false, "\u{1b}\r"),
        (LogicalKey::Character("[".into()), ModifiersState::CONTROL, false, "\u{1b}"),
        (LogicalKey::Character(" ".into()), ModifiersState::CONTROL, false, "\0"),
    ];
    for (key, modifiers, application, bytes) in wanted {
        assert_eq!(press(key.clone(), modifiers, application).as_deref(), Some(bytes), "{key:?}");
    }
}

#[test]
fn window_shortcuts_stay_with_the_window() {
    use crate::events::{Key, LogicalKey, ModifiersState};
    let mut command = crate::events::KeyEvent::new(LogicalKey::Character("t".into()), crate::events::KeyPhase::Down);
    command.modifiers = ModifiersState::META;
    command.text = Some("t".to_string());
    assert_eq!(super::key_payload(&command, false), None);
    assert_eq!(press(LogicalKey::Character("`".into()), ModifiersState::CONTROL, false), None);
    assert_eq!(press(LogicalKey::Named(Key::Tab), ModifiersState::CONTROL, false), None);
}
