use super::*;

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

#[test]
fn srgb_roundtrip_at_extremes() {
    for c in [0u8, 1, 64, 128, 200, 254, 255] {
        let back = linear_channel_to_srgb(srgb_channel_to_linear(c));
        assert!(back.abs_diff(c) <= 1, "{c} -> {back}");
    }
}

#[test]
fn theme_snapshot_resolves_scale_step_tokens() {
    let theme = Theme::default();
    let snap = ThemeSnapshot::from_theme(&theme);
    for (token, expected) in [
        ("accent-9", theme.accent_scale.step(9)),
        ("success-3", theme.success.step(3)),
        ("error-11", theme.error.step(11)),
        ("gray-2", theme.gray.step(2)),
        ("warn-7", theme.warn.step(7)),
        ("info-5", theme.info.step(5)),
    ] {
        let resolved = snap.resolve(ColorKey::parse(token));
        assert_eq!(resolved, expected, "{token:?}");
        assert_ne!(resolved, theme.missing_token_fallback, "{token:?}");
    }
}

#[test]
fn lerp_endpoints_midpoint_and_clamping() {
    let a = rgba(64, 128, 200, 255);
    let b = rgba(200, 64, 32, 128);
    assert_eq!(lerp_rgba8(a, b, 0.0), a);
    assert_eq!(lerp_rgba8(a, b, 1.0), b);
    let mid = lerp_rgba8(a, b, 0.5);
    for (m, x, y) in [(mid.r, a.r, b.r), (mid.g, a.g, b.g), (mid.b, a.b, b.b)] {
        assert!(m >= x.min(y) && m <= x.max(y));
    }
    assert!(mid.a.abs_diff(192) <= 1, "alpha midpoint drift: {}", mid.a);
    let (black, white) = (rgba(0, 0, 0, 0), rgba(255, 255, 255, 255));
    assert_eq!(lerp_rgba8(black, white, -0.5), black);
    assert_eq!(lerp_rgba8(black, white, 1.5), white);
}

#[test]
fn lerp_linear_space_is_brighter_than_srgb_at_midpoint() {
    let mid = lerp_rgba8(rgba(0, 0, 0, 255), rgba(255, 255, 255, 255), 0.5);
    assert!((180..=195).contains(&mid.r), "{}", mid.r);
}

#[test]
fn theme_snapshot_honours_runtime_token_overrides() {
    let pink = rgba(255, 0, 128, 255);
    let theme = Theme::default().with_token("accent-9", crate::style::ThemeValue::Color(pink));
    let snap = ThemeSnapshot::from_theme(&theme);
    assert_eq!(snap.resolve(ColorKey::parse("accent-9")), pink);
}
