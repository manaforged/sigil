use super::*;

#[test]
fn maps_logical_keys_without_case_normalising() {
    for (winit, sigil) in [
        (
            WinitKey::Named(WinitNamedKey::Enter),
            Key::Named(NamedKey::Enter),
        ),
        (
            WinitKey::Named(WinitNamedKey::Tab),
            Key::Named(NamedKey::Tab),
        ),
        (
            WinitKey::Named(WinitNamedKey::Escape),
            Key::Named(NamedKey::Escape),
        ),
        (WinitKey::Named(WinitNamedKey::F5), Key::Named(NamedKey::F5)),
        (
            WinitKey::Named(WinitNamedKey::ArrowUp),
            Key::Named(NamedKey::ArrowUp),
        ),
        (WinitKey::Named(WinitNamedKey::Space), Key::character(" ")),
        (WinitKey::Character("s".into()), Key::character("s")),
        (WinitKey::Character("S".into()), Key::character("S")),
    ] {
        assert_eq!(map_logical_key(winit), sigil);
    }
}

#[test]
fn maps_physical_keys_with_super_as_web_meta() {
    for (code, key) in [
        (WinitKeyCode::KeyA, PhysicalKey::KeyA),
        (WinitKeyCode::Digit1, PhysicalKey::Digit1),
        (WinitKeyCode::ArrowLeft, PhysicalKey::ArrowLeft),
        (WinitKeyCode::NumpadEnter, PhysicalKey::NumpadEnter),
        (WinitKeyCode::ShiftLeft, PhysicalKey::ShiftLeft),
        (WinitKeyCode::F12, PhysicalKey::F12),
        (WinitKeyCode::SuperLeft, PhysicalKey::MetaLeft),
        (WinitKeyCode::SuperRight, PhysicalKey::MetaRight),
    ] {
        assert_eq!(map_physical_key(WinitPhysicalKey::Code(code)), key);
    }
}
