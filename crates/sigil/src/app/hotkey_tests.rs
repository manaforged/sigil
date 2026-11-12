#![allow(clippy::unwrap_used)]

use global_hotkey::hotkey::HotKey;

use crate::events::{Key, Modifiers, Shortcut};

use super::to_hotkey;

#[test]
fn a_shortcut_maps_to_the_same_global_hotkey() {
    let shortcut = Shortcut::new(Key::character("k"), Modifiers::CTRL | Modifiers::SHIFT);
    let expected: HotKey = "shift+control+KeyK".parse().unwrap();
    assert_eq!(to_hotkey(&shortcut).unwrap(), expected);
}
