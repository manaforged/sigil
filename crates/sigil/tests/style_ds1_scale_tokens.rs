#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use sigil::style::scale::{
    amber_light, blue_dark, blue_light, cyan_light, green_light, ruby_light, slate_dark,
    slate_light,
};
use sigil::style::{StyleAtom, Theme, Token, Tone};

#[test]
fn role_step_tokens_resolve_to_scale_steps() {
    let theme = Theme::default();
    assert_eq!(theme.color(Tone::Neutral.step(2)), slate_light().step(2));
    assert_eq!(theme.color(Tone::Neutral.step(12)), slate_light().step(12));
    assert_eq!(theme.color(Tone::Primary.step(9)), blue_light().step(9));
    assert_eq!(theme.color(Tone::Success.step(3)), green_light().step(3));
    assert_eq!(theme.color(Tone::Warning.step(9)), amber_light().step(9));
    assert_eq!(theme.color(Tone::Error.step(9)), ruby_light().step(9));
    assert_eq!(theme.color(Tone::Info.step(3)), cyan_light().step(3));

    assert_eq!(theme.color(Tone::Success.step(11)), green_light().step(11));
    assert_eq!(theme.color(Tone::Error.step(11)), ruby_light().step(11));
}

#[test]
fn semantic_aliases_resolve_to_scale_steps() {
    let theme = Theme::default();
    assert_eq!(theme.color(Token::SurfaceSubtle), slate_light().step(3));
    assert_eq!(theme.color(Token::OnSurfaceMuted), slate_light().step(11));
    assert_eq!(theme.color(Token::OnSurface), slate_light().step(12));
}

#[test]
fn dark_theme_flips_every_scale_and_semantic_alias() {
    let theme = Theme::dark();
    assert_eq!(theme.color(Tone::Neutral.step(2)), slate_dark().step(2));
    assert_eq!(theme.color(Tone::Neutral.step(12)), slate_dark().step(12));
    assert_eq!(theme.color(Tone::Primary.step(9)), blue_dark().step(9));

    assert_eq!(theme.color(Token::Surface), slate_dark().step(2));
    assert_eq!(theme.color(Token::OnSurface), slate_dark().step(12));
    assert_eq!(theme.color(Token::Accent), blue_dark().step(9));
    assert_eq!(theme.color(Token::Border), slate_dark().step(7));

    assert_eq!(theme.color(Token::SurfaceSubtle), slate_dark().step(3));
    assert_eq!(theme.color(Token::OnSurfaceMuted), slate_dark().step(11));

    assert_ne!(theme.color(Token::Surface), slate_light().step(2));
    assert_ne!(theme.color(Token::OnSurface), slate_light().step(12));
}

#[test]
fn customizing_accent_scale_propagates_through_role_tokens() {
    let theme = Theme::default().tone(Tone::Primary, ruby_light());
    assert_eq!(theme.color(Tone::Primary.step(9)), ruby_light().step(9));
    assert_eq!(theme.color(Tone::Primary.step(3)), ruby_light().step(3));
    assert_eq!(theme.color(Token::Accent), ruby_light().step(9));
}

#[test]
fn tone_step_resolution_does_not_allocate() {
    let theme = Theme::default();
    let engine = sigil::style::StyleEngine::new(&theme);
    let atoms = [
        StyleAtom::BgToneStep(3),
        StyleAtom::FgToneStep(11),
        StyleAtom::BorderToneStep(6),
    ];
    let mut render = sigil::style::RenderStyle::default();
    engine.apply(&atoms, &mut render);
    let info = allocation_counter::measure(|| engine.apply(&atoms, &mut render));
    assert_eq!(info.count_total, 0, "{info:?}");
}
