use super::*;
use crate::style::ColorKey;
use crate::style::palette::Palette;
use crate::style::palette::perceived_brightness_u32;
use crate::style::variant::{FontFamilyHint, Variant};

#[test]
fn normal_baseline_locks_obsidian_plus_normal() {
    let theme = Theme::normal();
    let palette = Palette::obsidian();
    assert_eq!(theme.surface, palette.background);
    assert_eq!(theme.on_surface, palette.text);
    assert_eq!(theme.accent, palette.primary);
    assert_ne!(theme.accent_hover, theme.accent);
    assert_eq!(
        font_family_hint(Variant::Normal),
        FontFamilyHint::Proportional
    );
}

#[test]
fn terminal_baseline_locks_terminal_plus_terminal() {
    let theme = Theme::terminal();
    let palette = Palette::terminal();
    assert_eq!(theme.surface, palette.background);
    assert_eq!(theme.on_surface, palette.text);
    assert_eq!(theme.accent, palette.primary);
    assert_eq!(theme.surface_hover, theme.surface);
    assert_eq!(theme.surface_pressed, theme.surface);
    assert_eq!(theme.accent_hover, theme.accent);
    assert_eq!(
        font_family_hint(Variant::Terminal),
        FontFamilyHint::Monospace
    );
}

#[test]
fn every_palette_composes_with_every_variant() {
    let palettes = [
        Palette::aetherlight(),
        Palette::obsidian(),
        Palette::terminal(),
        Palette::voltglyph(),
        Palette::mythril(),
        Palette::frostkeep(),
        Palette::vellum(),
        Palette::embergrove(),
        Palette::verdigris(),
        Palette::faewine(),
        Palette::crystalline(),
    ];
    for palette in palettes {
        for variant in [Variant::Normal, Variant::Terminal] {
            let theme = Theme::from_palette(palette, variant);
            assert_ne!(theme.surface, theme.on_surface);
            assert_ne!(
                theme.accent,
                Rgba8 {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 0
                }
            );
        }
    }
}

#[test]
fn on_accent_contrasts_with_dark_and_bright_accents() {
    for accent in [rgb(0x20, 0x22, 0x2E), rgb(0xF6, 0xE9, 0xC4)] {
        let theme = Theme::from_palette(Palette::spellforge(accent), Variant::Normal);
        let accent_brightness = perceived_brightness_u32(theme.accent);
        let on_accent_brightness = perceived_brightness_u32(theme.on_accent);
        let contrast = accent_brightness.abs_diff(on_accent_brightness);
        assert!(
            contrast > 7_500,
            "on_accent too close to accent: accent_b={accent_brightness}, \
             on_accent_b={on_accent_brightness}, contrast={contrast}",
        );
    }
}

#[test]
fn console_dark_resolves_shadcn_tokens_to_captured_srgb() {
    let t = Theme::console_dark();
    for (name, expected) in [
        ("background", rgb(10, 10, 10)),
        ("card", rgb(23, 23, 23)),
        ("primary", rgb(229, 229, 229)),
        ("muted-foreground", rgb(161, 161, 161)),
        ("destructive", rgb(255, 100, 103)),
        ("sidebar-primary", rgb(20, 71, 230)),
        ("accent", rgb(38, 38, 38)),
        (
            "border",
            Rgba8 {
                r: 245,
                g: 255,
                b: 255,
                a: 26,
            },
        ),
    ] {
        assert_eq!(t.resolve_color(ColorKey::parse(name)), expected, "{name}");
    }
}

#[test]
fn dark_built_in_accent_unaffected_by_token_first_resolution() {
    let t = Theme::dark();
    assert_eq!(t.resolve_color(ColorKey::parse("accent")), t.accent);
    assert_ne!(t.resolve_color(ColorKey::parse("accent")), rgb(38, 38, 38));
}

#[test]
fn runtime_token_rebinding_accent_wins_over_builtin() {
    let gold = rgb(0xE8, 0xC0, 0x60);
    let navy = rgb(0x0D, 0x12, 0x20);
    let t = Theme::dark()
        .with_token("accent", ThemeValue::Color(gold))
        .with_token("on-accent", ThemeValue::Color(navy));
    assert_eq!(t.resolve_color(ColorKey::parse("accent")), gold);
    assert_eq!(t.accent, gold);
    assert_eq!(t.accent_scale.step(9), gold);
    assert_eq!(t.accent_scale.on_solid, navy);
    assert_ne!(
        t.resolve_color(ColorKey::parse("accent")),
        Theme::dark().accent
    );
}
