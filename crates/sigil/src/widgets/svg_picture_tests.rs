#![allow(
    clippy::cast_precision_loss,
    clippy::panic,
    reason = "a test fails loudly, and image sides are small"
)]

use super::{MAX_SIDE, parse, raster};

#[test]
fn a_thin_wide_svg_rasters_within_the_side_cap() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100000" height="2"><rect width="100000" height="2"/></svg>"#;
    let tree = parse(svg).expect("tree");
    let image = raster(1, &tree, (100_000.0, 2.0)).expect("image");
    assert!(
        image.width() as f32 <= MAX_SIDE && image.height() as f32 <= MAX_SIDE,
        "rastered at {}x{}",
        image.width(),
        image.height()
    );
}

#[test]
fn a_panicking_renderer_yields_no_picture() {
    assert!(super::picture("panics", |_| panic!("renderer bug")).is_none());
}
