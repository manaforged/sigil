#![allow(clippy::unwrap_used, clippy::panic)]

use super::*;

fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

#[test]
fn srgb_roundtrip_at_extremes() {
    for c in [0u8, 1, 64, 128, 200, 254, 255] {
        let lin = srgb_channel_to_linear(c);
        let back = linear_channel_to_srgb(lin);
        assert!(
            back.abs_diff(c) <= 1,
            "sRGB roundtrip drift > 1 LSB for {c}: lin={lin}, back={back}",
        );
    }
}

#[test]
fn theme_snapshot_resolves_scale_step_tokens() {
    // Regression for adversarial review 2026-05-21:
    // animated transitions of `bg-accent-9`, `bg-success-3`, etc.
    // previously fell through to `missing_token_fallback`. They
    // must now resolve to the same colour the static
    // `Theme::resolve_color` path picks.
    let theme = Theme::default();
    let snap = ThemeSnapshot::from_theme(&theme);
    let cases = [
        ("accent-9", theme.accent_scale.step(9)),
        ("success-3", theme.success.step(3)),
        ("error-11", theme.error.step(11)),
        ("gray-2", theme.gray.step(2)),
        ("warn-7", theme.warn.step(7)),
        ("info-5", theme.info.step(5)),
    ];
    for (token, expected) in cases {
        let resolved = snap.resolve(token);
        assert_eq!(
            resolved, expected,
            "ThemeSnapshot::resolve({token:?}) returned {resolved:?}, \
             expected scale-step color {expected:?}",
        );
        assert_ne!(
            resolved, snap.missing_token_fallback,
            "{token:?} silently fell back to magenta — scale-step path broken",
        );
    }
}

#[test]
fn theme_snapshot_unknown_token_still_falls_back() {
    let theme = Theme::default();
    let snap = ThemeSnapshot::from_theme(&theme);
    assert_eq!(
        snap.resolve("not-a-real-token-name"),
        snap.missing_token_fallback,
    );
}

#[test]
fn lerp_endpoint_t0_equals_a() {
    let a = rgba(64, 128, 200, 255);
    let b = rgba(200, 64, 32, 128);
    let mixed = lerp_rgba8(a, b, 0.0);
    assert_eq!(mixed, a, "t=0 must equal endpoint a");
}

#[test]
fn lerp_endpoint_t1_equals_b() {
    let a = rgba(64, 128, 200, 255);
    let b = rgba(200, 64, 32, 128);
    let mixed = lerp_rgba8(a, b, 1.0);
    assert_eq!(mixed, b, "t=1 must equal endpoint b");
}

#[test]
fn lerp_midpoint_between_endpoints() {
    // Midpoint of (64, 128, 200) and (200, 64, 32) is between the
    // endpoints per-channel — linear-light interpolation tolerates
    // a ~2 LSB drift after the sRGB roundtrip.
    let a = rgba(64, 128, 200, 255);
    let b = rgba(200, 64, 32, 128);
    let mid = lerp_rgba8(a, b, 0.5);
    // Each channel sits strictly between the endpoint values (or
    // exactly equal when endpoints coincide).
    assert!(mid.r >= a.r.min(b.r) && mid.r <= a.r.max(b.r));
    assert!(mid.g >= a.g.min(b.g) && mid.g <= a.g.max(b.g));
    assert!(mid.b >= a.b.min(b.b) && mid.b <= a.b.max(b.b));
    // Alpha midpoint of 255 and 128 = 192 ± 1.
    assert!(mid.a.abs_diff(192) <= 1, "alpha midpoint drift: {}", mid.a);
}

#[test]
fn lerp_t_clamps_outside_unit_interval() {
    let a = rgba(0, 0, 0, 0);
    let b = rgba(255, 255, 255, 255);
    assert_eq!(lerp_rgba8(a, b, -0.5), a, "t<0 clamps to 0");
    assert_eq!(lerp_rgba8(a, b, 1.5), b, "t>1 clamps to 1");
}

#[test]
fn lerp_linear_space_is_brighter_than_srgb_at_midpoint() {
    // Linear-light midpoint of black and white (which is sRGB
    // 188) is visibly brighter than the naive sRGB midpoint (128).
    // This is the contract the renderer relies on — verifying it
    // catches accidental sRGB-space lerp regressions.
    let mid = lerp_rgba8(rgba(0, 0, 0, 255), rgba(255, 255, 255, 255), 0.5);
    assert!(
        mid.r >= 180 && mid.r <= 195,
        "linear-light midpoint of black+white must sit near 188 sRGB (got {})",
        mid.r,
    );
}
