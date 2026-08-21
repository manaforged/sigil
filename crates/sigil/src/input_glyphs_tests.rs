use super::{InputAction, InputDevice, glyph_id};

#[test]
fn confirm_resolves_per_device() {
    assert_eq!(glyph_id(InputAction::Confirm, InputDevice::Xbox), "A");
    assert_eq!(
        glyph_id(InputAction::Confirm, InputDevice::Playstation),
        "✕"
    );
    assert_eq!(glyph_id(InputAction::Confirm, InputDevice::Switch), "B");
    assert_eq!(
        glyph_id(InputAction::Confirm, InputDevice::Keyboard),
        "Enter"
    );
}

#[test]
fn cancel_resolves_per_device() {
    assert_eq!(glyph_id(InputAction::Cancel, InputDevice::Xbox), "B");
    assert_eq!(glyph_id(InputAction::Cancel, InputDevice::Playstation), "○");
    assert_eq!(glyph_id(InputAction::Cancel, InputDevice::Switch), "A");
    assert_eq!(glyph_id(InputAction::Cancel, InputDevice::Keyboard), "Esc");
}

#[test]
fn dpad_resolves_to_arrows_on_controllers_wasd_on_keyboard() {
    for device in [
        InputDevice::Xbox,
        InputDevice::Playstation,
        InputDevice::Switch,
    ] {
        assert_eq!(glyph_id(InputAction::Up, device), "↑");
        assert_eq!(glyph_id(InputAction::Down, device), "↓");
        assert_eq!(glyph_id(InputAction::Left, device), "←");
        assert_eq!(glyph_id(InputAction::Right, device), "→");
    }
    assert_eq!(glyph_id(InputAction::Up, InputDevice::Keyboard), "W");
    assert_eq!(glyph_id(InputAction::Down, InputDevice::Keyboard), "S");
    assert_eq!(glyph_id(InputAction::Left, InputDevice::Keyboard), "A");
    assert_eq!(glyph_id(InputAction::Right, InputDevice::Keyboard), "D");
}

#[test]
fn jump_attack_interact_cover_every_device() {
    for device in [
        InputDevice::Xbox,
        InputDevice::Playstation,
        InputDevice::Switch,
        InputDevice::Keyboard,
    ] {
        assert!(!glyph_id(InputAction::Jump, device).is_empty());
        assert!(!glyph_id(InputAction::Attack, device).is_empty());
        assert!(!glyph_id(InputAction::Interact, device).is_empty());
    }
}

#[test]
fn menu_resolves_per_device() {
    assert_eq!(glyph_id(InputAction::Menu, InputDevice::Xbox), "☰");
    assert_eq!(glyph_id(InputAction::Menu, InputDevice::Playstation), "≡");
    assert_eq!(glyph_id(InputAction::Menu, InputDevice::Switch), "+");
    assert_eq!(glyph_id(InputAction::Menu, InputDevice::Keyboard), "Tab");
}
