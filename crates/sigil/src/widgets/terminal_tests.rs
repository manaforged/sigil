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
    use crate::events::{Key, KeyEvent, Modifiers, NamedKey};
    let pressed = |key: Key, modifiers: Modifiers| {
        let mut event = KeyEvent::new(key, crate::events::KeyPhase::Down);
        event.modifiers = modifiers;
        super::key_payload(&event, false)
    };
    let none = Modifiers::empty();
    assert_eq!(
        pressed(Key::Named(NamedKey::ArrowUp), none).as_deref(),
        Some("\u{1b}[A")
    );
    assert_eq!(
        pressed(Key::Named(NamedKey::Enter), none).as_deref(),
        Some("\r")
    );
    assert_eq!(
        pressed(Key::Named(NamedKey::Escape), none).as_deref(),
        Some("\u{1b}")
    );
    assert_eq!(
        pressed(Key::Character("c".into()), Modifiers::CTRL).as_deref(),
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

fn press(
    key: crate::events::Key,
    modifiers: crate::events::Modifiers,
    application: bool,
) -> Option<String> {
    let mut event = crate::events::KeyEvent::new(key, crate::events::KeyPhase::Down);
    event.modifiers = modifiers;
    super::key_payload(&event, application)
}

#[test]
fn a_full_screen_program_gets_the_keys_it_expects() {
    use crate::events::{Key, Modifiers, NamedKey};
    let none = Modifiers::empty();
    let named = Key::Named;
    let wanted = [
        (named(NamedKey::ArrowUp), none, true, "\u{1b}OA"),
        (
            named(NamedKey::ArrowUp),
            Modifiers::CTRL,
            true,
            "\u{1b}[1;5A",
        ),
        (named(NamedKey::Tab), Modifiers::SHIFT, false, "\u{1b}[Z"),
        (named(NamedKey::F1), none, false, "\u{1b}OP"),
        (named(NamedKey::F5), none, false, "\u{1b}[15~"),
        (named(NamedKey::PageUp), none, false, "\u{1b}[5~"),
        (named(NamedKey::Enter), Modifiers::SHIFT, false, "\u{1b}\r"),
        (Key::Character("[".into()), Modifiers::CTRL, false, "\u{1b}"),
        (Key::Character(" ".into()), Modifiers::CTRL, false, "\0"),
    ];
    for (key, modifiers, application, bytes) in wanted {
        assert_eq!(
            press(key.clone(), modifiers, application).as_deref(),
            Some(bytes),
            "{key:?}"
        );
    }
}

#[test]
fn window_shortcuts_stay_with_the_window() {
    use crate::events::{Key, Modifiers, NamedKey};
    let mut command =
        crate::events::KeyEvent::new(Key::Character("t".into()), crate::events::KeyPhase::Down);
    command.modifiers = Modifiers::META;
    command.text = Some("t".to_string());
    assert_eq!(super::key_payload(&command, false), None);
    assert_eq!(
        press(Key::Character("`".into()), Modifiers::CTRL, false),
        None
    );
    assert_eq!(
        press(Key::Named(NamedKey::Tab), Modifiers::CTRL, false),
        None
    );
}
