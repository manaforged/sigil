#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::kernel::Cx;

#[test]
fn cx_scale_defaults_to_one_for_launch() {
    let cx = Cx::new_root();

    assert!((cx.scale() - 1.0).abs() < f32::EPSILON);
}
