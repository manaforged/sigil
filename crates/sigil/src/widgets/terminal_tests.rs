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
        super::key_payload(&event)
    };
    let none = ModifiersState::empty();
    assert_eq!(pressed(LogicalKey::Named(Key::ArrowUp), none).as_deref(), Some("\u{1b}[A"));
    assert_eq!(pressed(LogicalKey::Named(Key::Enter), none).as_deref(), Some("\r"));
    assert_eq!(pressed(LogicalKey::Named(Key::Escape), none).as_deref(), Some("\u{1b}"));
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
            assert!(!node.a11y.focusable, "a node inside the terminal takes focus away from it");
            stack.extend(node.children().iter().copied());
        }
        assert!(cx.get(root).expect("terminal root").a11y.focusable);
    });
}
