#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::hal::Scale;
use sigil::kernel::Cx;

#[test]
fn cx_scale_defaults_to_one() {
    let cx = Cx::new_root();

    assert_eq!(cx.scale(), Scale::ONE);
}

#[test]
fn set_scale_updates_value() {
    let mut cx = Cx::new_root();

    cx.set_scale(Scale::new(2.0));

    assert_eq!(cx.scale(), Scale::new(2.0));
}

#[test]
fn set_scale_bumps_generation_when_changed() {
    let mut cx = Cx::new_root();
    let generation = cx.mutation_generation();

    cx.set_scale(Scale::new(2.0));

    assert_ne!(cx.mutation_generation(), generation);
}

#[test]
fn set_scale_is_noop_when_unchanged() {
    let mut cx = Cx::new_root();
    let generation = cx.mutation_generation();

    cx.set_scale(Scale::ONE);

    assert_eq!(cx.mutation_generation(), generation);
}

#[test]
fn set_scale_with_nonpositive_factor_clamps_via_scale_new() {
    let mut cx = Cx::new_root();
    cx.set_scale(Scale::new(2.0));

    cx.set_scale(Scale::new(0.0));

    assert_eq!(cx.scale(), Scale::ONE);
}
