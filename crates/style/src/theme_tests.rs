    use super::*;
    use crate::palette::Palette;
    use crate::variant::{FontFamilyHint, Variant};

    /// `Theme::normal()` is locked: Obsidian palette + Normal variant.
    /// Any drift in this composition fails the dual-baseline visual
    /// regression contract.
    #[test]
    fn normal_baseline_locks_obsidian_plus_normal() {
        let theme = Theme::normal();
        let palette = Palette::obsidian();
        assert_eq!(theme.surface, palette.background);
        assert_eq!(theme.on_surface, palette.text);
        assert_eq!(theme.accent, palette.primary);
        // Normal variant has hover shift, so accent_hover differs from
        // accent.
        assert_ne!(theme.accent_hover, theme.accent);
        // Normal variant uses proportional font.
        assert_eq!(
            font_family_hint(Variant::Normal),
            FontFamilyHint::Proportional
        );
    }

    /// `Theme::terminal()` is locked: Terminal palette + Terminal
    /// variant. Saturated green-on-black with no hover lift.
    #[test]
    fn terminal_baseline_locks_terminal_plus_terminal() {
        let theme = Theme::terminal();
        let palette = Palette::terminal();
        assert_eq!(theme.surface, palette.background);
        assert_eq!(theme.on_surface, palette.text);
        assert_eq!(theme.accent, palette.primary);
        // Terminal variant suppresses hover lift — surface_hover,
        // surface_pressed and accent_hover must equal their bases.
        assert_eq!(theme.surface_hover, theme.surface);
        assert_eq!(theme.surface_pressed, theme.surface);
        assert_eq!(theme.accent_hover, theme.accent);
        // Terminal variant uses monospace font.
        assert_eq!(
            font_family_hint(Variant::Terminal),
            FontFamilyHint::Monospace
        );
    }

    /// Every shipped arcane palette composes with both variants without
    /// panicking and produces a usable theme.
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

    /// `on_accent` must contrast with the accent itself, not just
    /// fall back to `palette.background`. A `Spellforge` palette
    /// derived from a *dark* accent on the dark default surface
    /// previously produced dark-on-dark text-on-accent — unreadable.
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
        // Demand at least 30% of the full brightness range (0..=25500)
        // between accent and its on-accent text. The buggy version
        // produced contrast ≈ 0 for dark-on-dark.
        assert!(
            contrast > 7_500,
            "on_accent too close to accent: accent_b={accent_brightness}, \
             on_accent_b={on_accent_brightness}, contrast={contrast}",
        );
    }

    /// Same demand for a bright accent on a bright palette — the
    /// symmetric case the prior implementation also handled only by
    /// accident.
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
