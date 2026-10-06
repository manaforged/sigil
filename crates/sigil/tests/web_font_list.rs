#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::text::{WebFont, WebFontScript, set_web_fallbacks, web_fallbacks};

#[test]
fn the_web_font_list_is_pinned_and_replaceable() {
    let defaults = web_fallbacks();
    assert!(!defaults.is_empty());
    assert!(
        defaults.iter().all(|font| !font.url.contains("@main")),
        "a default font URL follows a moving branch"
    );
    set_web_fallbacks([WebFont::new(WebFontScript::Cjk, "fonts/cjk.otf", 0)]);
    assert_eq!(web_fallbacks().len(), 1);
    set_web_fallbacks([]);
    assert!(web_fallbacks().is_empty());
    set_web_fallbacks(WebFont::noto_defaults());
}
