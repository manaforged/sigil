use sigil::kernel::Cx;

#[test]
fn cx_scale_defaults_to_one() {
    let cx = Cx::new_root();

    assert!((cx.scale() - 1.0).abs() < f32::EPSILON);
}

#[test]
fn set_scale_updates_value() {
    let mut cx = Cx::new_root();

    cx.set_scale(2.0);

    assert!((cx.scale() - 2.0).abs() < f32::EPSILON);
}

#[test]
fn set_scale_bumps_generation_when_changed() {
    let mut cx = Cx::new_root();
    let generation = cx.mutation_generation();

    cx.set_scale(2.0);

    assert_ne!(cx.mutation_generation(), generation);
}

#[test]
fn set_scale_is_noop_when_unchanged() {
    let mut cx = Cx::new_root();
    let generation = cx.mutation_generation();

    cx.set_scale(1.0);

    assert_eq!(cx.mutation_generation(), generation);
}

#[test]
fn set_scale_with_nonpositive_factor_clamps() {
    let mut cx = Cx::new_root();
    cx.set_scale(2.0);

    cx.set_scale(0.0);

    assert!((cx.scale() - 1.0).abs() < f32::EPSILON);
}
