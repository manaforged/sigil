#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::raster::to_ts_color;
use super::*;

const fn rgb(r: u8, g: u8, b: u8) -> Rgba8 {
    Rgba8 { r, g, b, a: 255 }
}

fn triangle(color: Rgba8) -> PathNode {
    path(32, 32)
        .move_to(16.0, 2.0)
        .line_to(30.0, 30.0)
        .line_to(2.0, 30.0)
        .close()
        .fill(color)
}

fn diagonal() -> PathNode {
    path(16, 16).move_to(0.0, 0.0).line_to(16.0, 16.0)
}

#[test]
fn fills_a_triangle_in_color() {
    let blue = rgb(40, 90, 230);
    let node = triangle(blue);
    let source = node
        .rasterize(&Theme::light())
        .expect("triangle rasterizes");
    assert_eq!(source.width(), 32);
    assert_eq!(source.height(), 32);
    let again = triangle(blue).rasterize(&Theme::light()).unwrap();
    assert_eq!(
        source.id(),
        again.id(),
        "same commands reuse the CPU raster"
    );
    let mut pixmap = Pixmap::new(32, 32).unwrap();
    let ts_path = build_ts_path(&node.cmds).unwrap();
    let mut paint = Paint::default();
    paint.set_color(to_ts_color(blue));
    paint.anti_alias = true;
    pixmap.fill_path(
        &ts_path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
    let center = pixmap.pixel(16, 20).unwrap().demultiply();
    assert!(
        center.alpha() > 200 && center.blue() > center.red(),
        "triangle interior must be opaque blue, got {center:?}"
    );
}

#[test]
fn empty_path_yields_no_image() {
    assert!(
        path(16, 16)
            .fill(rgb(0, 0, 0))
            .rasterize(&Theme::light())
            .is_none()
    );
}

#[test]
fn fit_rasters_two_x_and_pins_logical_box() {
    let node = path(16, 16)
        .move_to(2.0, 2.0)
        .line_to(14.0, 14.0)
        .stroke(rgb(255, 255, 255), 1.35)
        .fit();
    let source = node.rasterize(&Theme::light()).expect("stroke rasterizes");
    assert_eq!(source.width(), 32, "2x pixmap");
    assert_eq!(source.height(), 32);
    assert!(node.layout.is_some(), "fit pins a logical box");
}

#[test]
fn intern_key_distinguishes_gradient_fills() {
    let (red, blue) = (rgb(255, 0, 0), rgb(0, 0, 255));
    let a = diagonal().fill_linear((0.0, 0.0), (16.0, 0.0), vec![(0.0, red), (1.0, blue)]);
    let b = diagonal().fill_linear((0.0, 0.0), (16.0, 0.0), vec![(0.0, blue), (1.0, red)]);
    assert_ne!(
        a.intern_key(a.fill.as_ref(), None),
        b.intern_key(b.fill.as_ref(), None),
        "different gradient stops must not share a raster"
    );
}
