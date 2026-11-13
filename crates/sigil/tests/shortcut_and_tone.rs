use sigil::events::{Modifiers, NamedKey, Shortcut};
use sigil::prelude::*;

#[test]
fn shortcut_label_uses_symbols_on_mac_and_words_elsewhere() {
    let palette = Shortcut::new(Key::character("k"), Modifiers::META | Modifiers::SHIFT);
    assert_eq!(palette.label_for(true), "⇧⌘K");
    assert_eq!(palette.label_for(false), "Shift+Super+K");
    let save = Shortcut::ctrl(Key::character("s"));
    assert_eq!(save.label_for(false), "Ctrl+S");
    assert_eq!(save.label_for(true), "⌃S");
    assert_eq!(Shortcut::plain(NamedKey::Enter).label_for(true), "↵");
    assert_eq!(Shortcut::plain(NamedKey::Escape).label_for(false), "Esc");
}

#[test]
fn tone_tokens_follow_the_scale_steps() {
    assert_eq!(Tone::Success.fill(), Tone::Success.step(9));
    assert_eq!(Tone::Error.solid(), Tone::Error.step(11));
    assert_eq!(Tone::Warning.shade(), Tone::Warning.step(4));
    assert_eq!(Tone::Neutral.fill(), Tone::Neutral.step(9));
    assert_eq!(Tone::Info.step(99), Token::Step(Tone::Info, 12));
    assert_eq!(Tone::Info.step(0), Token::Step(Tone::Info, 1));
}
