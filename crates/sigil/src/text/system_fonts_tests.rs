use super::*;

#[test]
fn macos_system_families_resolve_real_weight_faces() {
    let fonts = load();
    let sans = fonts.sans.expect("macOS must expose the system UI face");
    let sans_face = sans.select(400);
    assert!(
        sans_face.has_weight_axis(),
        "system UI sans should carry a wght axis"
    );
    assert_ne!(
        sans_face.variation_coords(400, 14.0),
        sans_face.variation_coords(700, 14.0),
        "regular and bold sans must resolve to different locations",
    );

    let mono = fonts
        .mono
        .expect("macOS must expose a system monospace face");
    let regular = mono.select(400);
    let bold = mono.select(700);
    assert!(
        regular.id() != bold.id()
            || regular.variation_coords(400, 14.0) != bold.variation_coords(700, 14.0),
        "regular and bold mono must resolve to distinct faces or locations",
    );
}

#[test]
fn cjk_script_fallback_covers_han_and_hangul() {
    let faces = script_fallbacks(Script::Cjk);
    assert!(!faces.is_empty(), "macOS must expose a CJK face");
    assert!(faces.iter().any(|font| font.covers("東京証券")));
    assert!(faces.iter().any(|font| font.covers("서울")));
}

#[test]
fn arabic_script_fallback_covers_arabic() {
    let faces = script_fallbacks(Script::Arabic);
    assert!(faces.iter().any(|font| font.covers("مصرف")));
}
