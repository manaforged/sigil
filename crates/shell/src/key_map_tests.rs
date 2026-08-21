use super::*;

#[test]
fn maps_named_logical_keys() {
    assert_eq!(
        map_logical_key(WinitKey::Named(WinitNamedKey::Enter)),
        LogicalKey::Named(Key::Enter)
    );
    assert_eq!(
        map_logical_key(WinitKey::Named(WinitNamedKey::Tab)),
        LogicalKey::Named(Key::Tab)
    );
    assert_eq!(
        map_logical_key(WinitKey::Named(WinitNamedKey::Escape)),
        LogicalKey::Named(Key::Escape)
    );
    assert_eq!(
        map_logical_key(WinitKey::Named(WinitNamedKey::F5)),
        LogicalKey::Named(Key::F5)
    );
    assert_eq!(
        map_logical_key(WinitKey::Named(WinitNamedKey::ArrowUp)),
        LogicalKey::Named(Key::ArrowUp)
    );
}

#[test]
fn maps_space_named_key_to_character_literal() {
    // keyboard-types has no NamedKey::Space; it belongs on the
    // character stream per the W3C spec.
    assert_eq!(
        map_logical_key(WinitKey::Named(WinitNamedKey::Space)),
        LogicalKey::character(" ")
    );
}

#[test]
fn maps_character_logical_keys_without_case_normalising() {
    assert_eq!(
        map_logical_key(WinitKey::Character("s".into())),
        LogicalKey::character("s")
    );
    assert_eq!(
        map_logical_key(WinitKey::Character("S".into())),
        LogicalKey::character("S")
    );
}

#[test]
fn maps_physical_keys_used_by_shortcuts() {
    assert_eq!(
        map_physical_key(WinitPhysicalKey::Code(WinitKeyCode::KeyA)),
        PhysicalKey::KeyA
    );
    assert_eq!(
        map_physical_key(WinitPhysicalKey::Code(WinitKeyCode::Digit1)),
        PhysicalKey::Digit1
    );
    assert_eq!(
        map_physical_key(WinitPhysicalKey::Code(WinitKeyCode::ArrowLeft)),
        PhysicalKey::ArrowLeft
    );
    assert_eq!(
        map_physical_key(WinitPhysicalKey::Code(WinitKeyCode::NumpadEnter)),
        PhysicalKey::NumpadEnter
    );
    assert_eq!(
        map_physical_key(WinitPhysicalKey::Code(WinitKeyCode::ShiftLeft)),
        PhysicalKey::ShiftLeft
    );
    assert_eq!(
        map_physical_key(WinitPhysicalKey::Code(WinitKeyCode::F12)),
        PhysicalKey::F12
    );
}

#[test]
fn maps_super_physical_keys_to_web_meta_names() {
    assert_eq!(
        map_physical_key(WinitPhysicalKey::Code(WinitKeyCode::SuperLeft)),
        PhysicalKey::MetaLeft
    );
    assert_eq!(
        map_physical_key(WinitPhysicalKey::Code(WinitKeyCode::SuperRight)),
        PhysicalKey::MetaRight
    );
}
