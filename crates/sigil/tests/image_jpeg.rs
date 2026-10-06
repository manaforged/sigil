#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::image_bytes;

const JPEG: &[u8] = include_bytes!("fixtures/images/red-4x4.jpg");

#[test]
fn image_bytes_decodes_a_jpeg() {
    assert!(image_bytes(JPEG).is_some(), "JPEG bytes did not decode");
}
