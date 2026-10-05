#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::events::{Key, ModifiersState, Shortcut};
use sigil::prelude::*;

#[test]
fn shortcut_label_uses_symbols_on_mac_and_words_elsewhere() {
    let palette = Shortcut::new(
        LogicalKey::character("k"),
        ModifiersState::META | ModifiersState::SHIFT,
    );
    assert_eq!(palette.label_for(true), "⇧⌘K");
    assert_eq!(palette.label_for(false), "Shift+Super+K");
    let save = Shortcut::ctrl(LogicalKey::character("s"));
    assert_eq!(save.label_for(false), "Ctrl+S");
    assert_eq!(save.label_for(true), "⌃S");
    assert_eq!(Shortcut::plain(Key::Enter).label_for(true), "↵");
    assert_eq!(Shortcut::plain(Key::Escape).label_for(false), "Esc");
}

#[test]
fn tone_tokens_follow_the_scale_steps() {
    assert_eq!(Tone::Success.fill(), Token::Success9);
    assert_eq!(Tone::Error.solid(), Token::Error11);
    assert_eq!(Tone::Warn.shade(), Token::Warn4);
    assert_eq!(Tone::Neutral.fill(), Token::Gray9);
    assert_eq!(Tone::Info.token(99), Token::OnSurface);
}
