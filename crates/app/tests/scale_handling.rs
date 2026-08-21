#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Scale handling regression coverage for the launch stack.

#![allow(missing_docs)]

use sigil_hal::Scale;
use sigil_kernel::Cx;

#[test]
fn cx_scale_defaults_to_one_for_launch() {
    let cx = Cx::new_root();

    assert_eq!(cx.scale(), Scale::ONE);
}
