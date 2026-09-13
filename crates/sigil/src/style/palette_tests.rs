use super::hsv::rgb_to_hsv;
use super::*;

#[test]
fn every_palette_distinguishes_text_and_background() {
    for palette in [
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
    ] {
        assert_ne!(
            palette.text, palette.background,
            "{palette:?}: text and background must differ"
        );
    }
}

#[test]
fn terminal_palette_matches_baseline_spec() {
    let palette = Palette::terminal();
    assert_eq!(palette.background, rgb(0x0A, 0x0A, 0x0A));
    assert_eq!(palette.text, rgb(0x33, 0xFF, 0x33));
    assert_eq!(palette.primary, rgb(0x33, 0xFF, 0x33));
    assert_eq!(palette.success, rgb(0x33, 0xFF, 0x33));
    assert_eq!(palette.warning, rgb(0xFF, 0xAA, 0x00));
    assert_eq!(palette.danger, rgb(0xFF, 0x33, 0x33));
}

#[test]
fn spellforge_picks_light_surface_for_dark_accent() {
    let dark_accent = rgb(0x20, 0x10, 0x40);
    let palette = Palette::spellforge(dark_accent);
    let bg_lum = perceived_brightness(palette.background);
    let text_lum = perceived_brightness(palette.text);
    assert!(
        bg_lum > text_lum,
        "Dark accent should yield light bg + dark text \
         (bg_lum={bg_lum}, text_lum={text_lum})"
    );
    assert_eq!(palette.primary, dark_accent);
}

#[test]
fn spellforge_picks_dark_surface_for_bright_accent() {
    let bright_accent = rgb(0xFF, 0xC8, 0x60);
    let palette = Palette::spellforge(bright_accent);
    let bg_lum = perceived_brightness(palette.background);
    let text_lum = perceived_brightness(palette.text);
    assert!(
        text_lum > bg_lum,
        "Bright accent should yield dark bg + light text \
         (bg_lum={bg_lum}, text_lum={text_lum})"
    );
    assert_eq!(palette.primary, bright_accent);
}

#[test]
fn spellforge_role_colours_inherit_accent_saturation() {
    let saturated = rgb(0x33, 0x70, 0xFF);
    let palette = Palette::spellforge(saturated);
    let (_, s_success, _) = rgb_to_hsv(palette.success);
    let (_, s_warning, _) = rgb_to_hsv(palette.warning);
    let (_, s_danger, _) = rgb_to_hsv(palette.danger);
    assert!(s_success > 0.5, "success too desaturated: {s_success}");
    assert!(s_warning > 0.5, "warning too desaturated: {s_warning}");
    assert!(s_danger > 0.5, "danger too desaturated: {s_danger}");

    let grey = rgb(0x80, 0x80, 0x80);
    let grey_palette = Palette::spellforge(grey);
    let (_, s_grey_success, _) = rgb_to_hsv(grey_palette.success);
    assert!(
        s_grey_success < 0.05,
        "Grey accent should yield greyscale success, got s={s_grey_success}"
    );
}

#[test]
fn spellforge_role_colours_hit_canonical_hues() {
    let accents = [
        rgb(0xFF, 0x33, 0x33),
        rgb(0xFF, 0xC0, 0x33),
        rgb(0x33, 0xFF, 0x70),
        rgb(0x33, 0xC0, 0xFF),
        rgb(0x33, 0x70, 0xFF),
        rgb(0xC0, 0x33, 0xFF),
        rgb(0x80, 0x40, 0xC0),
        rgb(0xE6, 0xD8, 0xB0),
    ];

    for accent in accents {
        let palette = Palette::spellforge(accent);
        let (_, s_accent, v_accent) = rgb_to_hsv(accent);

        for (role_name, role_colour, target_hue) in [
            ("success", palette.success, 130.0_f32),
            ("warning", palette.warning, 45.0_f32),
            ("danger", palette.danger, 5.0_f32),
        ] {
            let (h, s, v) = rgb_to_hsv(role_colour);
            let wrapped_dist = (h - target_hue).abs().min((h - target_hue + 360.0).abs());
            let dist = wrapped_dist.min((h - target_hue - 360.0).abs());
            assert!(
                dist < 15.0,
                "{role_name} hue {h}° too far from canonical {target_hue}° \
                 for accent {accent:?}"
            );
            assert!(
                (s - s_accent).abs() < 0.06,
                "{role_name} saturation {s} drifted from accent saturation \
                 {s_accent} for accent {accent:?}"
            );
            assert!(
                (v - v_accent).abs() < 0.06,
                "{role_name} value {v} drifted from accent value {v_accent} \
                 for accent {accent:?}"
            );
        }
    }
}
