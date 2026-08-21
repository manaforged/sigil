use super::*;

#[test]
fn normal_has_radius_shadows_and_hover_lift() {
    let tokens = Variant::Normal.tokens();
    assert!(tokens.radius > 0);
    assert!(tokens.shadows);
    assert!(tokens.hover_lift);
    assert_eq!(tokens.font, FontFamilyHint::Proportional);
}

#[test]
fn terminal_is_sharp_monospace_flat_hover() {
    let tokens = Variant::Terminal.tokens();
    assert_eq!(tokens.radius, 0);
    assert!(!tokens.shadows);
    assert!(!tokens.hover_lift);
    assert_eq!(tokens.font, FontFamilyHint::Monospace);
}

#[test]
fn default_is_normal() {
    assert_eq!(Variant::default(), Variant::Normal);
}
