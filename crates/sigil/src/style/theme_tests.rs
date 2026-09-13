use super::*;
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
fn on_accent_contrasts_with_dark_accent_on_dark_palette() {
    let dark_accent = Rgba8 {
        r: 0x20,
        g: 0x22,
        b: 0x2E,
        a: 0xFF,
    };
    let palette = Palette::spellforge(dark_accent);
    let theme = Theme::from_palette(palette, Variant::Normal);
    let accent_brightness = perceived_brightness_u32(theme.accent);
    let on_accent_brightness = perceived_brightness_u32(theme.on_accent);
    let contrast = accent_brightness.abs_diff(on_accent_brightness);
    assert!(
        contrast > 7_500,
        "on_accent too close to accent: accent_b={accent_brightness}, \
             on_accent_b={on_accent_brightness}, contrast={contrast}",
    );
}

#[test]
fn on_accent_contrasts_with_bright_accent_on_bright_palette() {
    let bright_accent = Rgba8 {
        r: 0xF6,
        g: 0xE9,
        b: 0xC4,
        a: 0xFF,
    };
    let palette = Palette::spellforge(bright_accent);
    let theme = Theme::from_palette(palette, Variant::Normal);
    let accent_brightness = perceived_brightness_u32(theme.accent);
    let on_accent_brightness = perceived_brightness_u32(theme.on_accent);
    let contrast = accent_brightness.abs_diff(on_accent_brightness);
    assert!(
        contrast > 7_500,
        "on_accent too close to accent: accent_b={accent_brightness}, \
             on_accent_b={on_accent_brightness}, contrast={contrast}",
    );
}

#[test]
fn console_dark_resolves_shadcn_tokens_to_captured_srgb() {
    let t = Theme::console_dark();
    let rgba = |r, g, b, a| Rgba8 { r, g, b, a };
    assert_eq!(t.resolve_color("background"), rgba(10, 10, 10, 255));
    assert_eq!(t.resolve_color("card"), rgba(23, 23, 23, 255));
    assert_eq!(t.resolve_color("primary"), rgba(229, 229, 229, 255));
    assert_eq!(
        t.resolve_color("muted-foreground"),
        rgba(161, 161, 161, 255)
    );
    assert_eq!(t.resolve_color("destructive"), rgba(255, 100, 103, 255));
    assert_eq!(t.resolve_color("sidebar-primary"), rgba(20, 71, 230, 255));
    assert_eq!(t.resolve_color("accent"), rgba(38, 38, 38, 255));
    assert_eq!(t.resolve_color("border"), rgba(245, 255, 255, 26));
}

#[test]
fn dark_built_in_accent_unaffected_by_token_first_resolution() {
    let t = Theme::dark();
    assert_eq!(t.resolve_color("accent"), t.accent);
    assert_ne!(
        t.resolve_color("accent"),
        Rgba8 {
            r: 38,
            g: 38,
            b: 38,
            a: 255
        }
    );
}

#[test]
fn runtime_token_rebinding_accent_wins_over_builtin() {
    let gold = Rgba8 {
        r: 0xE8,
        g: 0xC0,
        b: 0x60,
        a: 255,
    };
    let navy = Rgba8 {
        r: 0x0D,
        g: 0x12,
        b: 0x20,
        a: 255,
    };
    let t = Theme::dark()
        .with_token("accent", ThemeValue::Color(gold))
        .with_token("on-accent", ThemeValue::Color(navy));
    assert_eq!(t.resolve_color("accent"), gold);
    assert_eq!(t.accent, gold);
    assert_eq!(t.accent_scale.step(9), gold);
    assert_eq!(t.accent_scale.on_solid, navy);
    assert_ne!(t.resolve_color("accent"), Theme::dark().accent);
}

#[test]
fn numbered_surfaces_and_line_resolve_on_default() {
    let theme = Theme::default();
    assert_eq!(theme.resolve_color("surface-0"), theme.gray.step(1));
    assert_eq!(theme.resolve_color("surface-1"), theme.surface);
    assert_eq!(theme.resolve_color("surface-2"), theme.surface_pressed);
    assert_eq!(
        theme.resolve_color("surface-3"),
        theme.surface_pressed_strong
    );
    assert_eq!(theme.resolve_color("line"), theme.border);
    let navy = Rgba8 {
        r: 0x0D,
        g: 0x12,
        b: 0x20,
        a: 255,
    };
    let themed = Theme::default().with_token("surface-1", ThemeValue::Color(navy));
    assert_eq!(themed.resolve_color("surface-1"), navy);
}
