#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::*;
use crate::geometry::Point;

const fn rgb(r: u8, g: u8, b: u8) -> Rgba8 {
    Rgba8 { r, g, b, a: 255 }
}

fn diagonal() -> PathNode {
    path(16, 16).move_to(0.0, 0.0).line_to(16.0, 16.0)
}

fn box16() -> Rect {
    Rect::new(Point::new(0.0, 0.0), 16.0, 16.0)
}

#[test]
fn empty_path_draws_nothing() {
    assert!(
        path(16, 16)
            .fill(rgb(0, 0, 0))
            .draw_path(box16(), &Theme::light())
            .is_none()
    );
}

#[test]
fn fit_pins_a_logical_box() {
    let node = diagonal().stroke(rgb(255, 255, 255), 1.35).fit();
    assert!(node.fit.is_some(), "fit pins a logical box");
}

#[test]
fn path_key_distinguishes_gradient_fills() {
    let (red, blue) = (rgb(255, 0, 0), rgb(0, 0, 255));
    let a = diagonal().fill_linear((0.0, 0.0), (16.0, 0.0), vec![(0.0, red), (1.0, blue)]);
    let b = diagonal().fill_linear((0.0, 0.0), (16.0, 0.0), vec![(0.0, blue), (1.0, red)]);
    let key = |node: &PathNode| node.draw_path(box16(), &Theme::light()).unwrap().key;
    assert_ne!(
        key(&a),
        key(&b),
        "different gradient stops must not share a raster"
    );
}
