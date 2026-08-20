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

    /// `console_dark()` resolves shadcn token names to the exact sRGB captured from the
    /// running the host app dark app. This is the parity
    /// contract: a ported React `className` keeps its token words and renders identically.
    #[test]
    fn console_dark_resolves_shadcn_tokens_to_captured_srgb() {
        let t = Theme::console_dark();
        let rgba = |r, g, b, a| Rgba8 { r, g, b, a };
        assert_eq!(t.resolve_color("background"), rgba(10, 10, 10, 255));
        assert_eq!(t.resolve_color("card"), rgba(23, 23, 23, 255));
        assert_eq!(t.resolve_color("primary"), rgba(229, 229, 229, 255));
        assert_eq!(t.resolve_color("muted-foreground"), rgba(161, 161, 161, 255));
        assert_eq!(t.resolve_color("destructive"), rgba(255, 100, 103, 255));
        assert_eq!(t.resolve_color("sidebar-primary"), rgba(20, 71, 230, 255));
        // Explicit token wins over the built-in `accent` (Sigil steel blue) — the
        // resolution reorder — and `border` carries its shadcn alpha (white at 10%).
        assert_eq!(t.resolve_color("accent"), rgba(38, 38, 38, 255));
        assert_eq!(t.resolve_color("border"), rgba(245, 255, 255, 26));
    }

    /// The resolution reorder must not disturb themes with an empty token map: `dark()`
    /// still resolves `accent` to its built-in steel-blue step, not a console value.
    #[test]
    fn dark_built_in_accent_unaffected_by_token_first_resolution() {
        let t = Theme::dark();
        assert_eq!(t.resolve_color("accent"), t.accent);
        assert_ne!(t.resolve_color("accent"), Rgba8 { r: 38, g: 38, b: 38, a: 255 });
    }

    #[test]
    fn runtime_token_rebinding_accent_wins_over_builtin() {
        let gold = Rgba8 { r: 0xE8, g: 0xC0, b: 0x60, a: 255 };
        let navy = Rgba8 { r: 0x0D, g: 0x12, b: 0x20, a: 255 };
        let t = Theme::dark()
            .with_token("accent", ThemeValue::Color(gold))
            .with_token("on-accent", ThemeValue::Color(navy));
        assert_eq!(t.resolve_color("accent"), gold);
        assert_eq!(t.accent, gold);
        assert_eq!(t.accent_scale.step(9), gold);
        assert_eq!(t.accent_scale.on_solid, navy);
        assert_ne!(t.resolve_color("accent"), Theme::dark().accent);
    }
