use super::*;

#[test]
fn step_indexes_from_one_through_twelve() {
    let scale = slate_light();
    assert_eq!(scale.step(1), rgb(0xFC, 0xFC, 0xFD));
    assert_eq!(scale.step(12), rgb(0x1C, 0x20, 0x24));
}

#[test]
fn step_clamps_zero_and_out_of_range() {
    let scale = slate_light();
    assert_eq!(scale.step(0), scale.step(1));
    assert_eq!(scale.step(13), scale.step(12));
    assert_eq!(scale.step(u8::MAX), scale.step(12));
}

#[test]
fn amber_dark_on_solid_is_dark_brown_not_pale_cream() {
    let amber = amber_dark();
    assert_eq!(amber.step(9), rgb(0xFF, 0xC5, 0x3D));
    assert_eq!(amber.on_solid, rgb(0x4F, 0x34, 0x22));
    assert_ne!(
        amber.on_solid,
        amber.step(12),
        "amber on_solid must NOT be step 12 (pale cream) — that fails WCAG AA"
    );
}

#[test]
fn light_scales_with_saturated_step_9_use_white_on_solid() {
    let white = rgb(0xFF, 0xFF, 0xFF);
    assert_eq!(iris_light().on_solid, white);
    assert_eq!(green_light().on_solid, white);
    assert_eq!(ruby_light().on_solid, white);
    assert_eq!(cyan_light().on_solid, white);
}
