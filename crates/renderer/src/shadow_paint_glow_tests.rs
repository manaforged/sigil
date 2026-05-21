// Included inside `shadow_paint::tests` by `shadow_paint.rs`.
// Tests for `ShadowKind::Glow`, `Shadow::glow`, and the WGSL glow branch.

#[test]
fn glow_shader_branch_present_and_ordered_before_inner() {
    // Glow guard must appear before the inner guard so the >= 1.5 check
    // fires before the >= 0.5 check.
    assert!(SHADOW_SHADER.contains("if shadow_kind >= 1.5"));
    let glow_pos = SHADOW_SHADER
        .find("if shadow_kind >= 1.5")
        .expect("glow branch");
    let inner_pos = SHADOW_SHADER
        .find("} else if shadow_kind >= 0.5")
        .expect("inner branch");
    assert!(
        glow_pos < inner_pos,
        "glow branch must precede inner branch"
    );
}

#[test]
fn glow_kind_as_f32_is_two() {
    assert!((super::ShadowKind::Glow.as_f32() - 2.0).abs() < f32::EPSILON);
}

#[test]
fn shadow_glow_constructor_sets_kind_and_zero_offset() {
    use style::Rgba8;
    let g = super::Shadow::glow(
        Rgba8 {
            r: 80,
            g: 200,
            b: 255,
            a: 128,
        },
        8.0,
        18.0,
    );
    assert_eq!(g.kind, super::ShadowKind::Glow);
    assert!((g.offset[0]).abs() < f32::EPSILON);
    assert!((g.offset[1]).abs() < f32::EPSILON);
    assert!((g.blur - 18.0).abs() < 1e-5);
    assert!((g.spread - 8.0).abs() < 1e-5);
    // alpha: 128/255 ≈ 0.502
    assert!((g.color[3] - 128.0 / 255.0).abs() < 1e-4);
}
