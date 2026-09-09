use crossterm::event::{KeyCode, KeyEvent as TerminalKeyEvent, KeyModifiers};

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
