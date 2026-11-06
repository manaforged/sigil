#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![cfg(feature = "kittest")]

use sigil::prelude::*;
use sigil::widgets::icon_svg;
use sigil_kittest::Harness;

const SIDE: u32 = 32;
const BLACK: Rgba8 = Rgba8 { r: 0, g: 0, b: 0, a: 255 };

fn frame(body: &'static str) -> image::RgbaImage {
    let app = move || {
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">{body}</svg>"#
        );
        column().bg(BLACK).child(icon_svg(svg).px(SIDE as f32))
    };
    let mut ui = Harness::new(app).pixels(SIDE, SIDE);
    ui.image().expect("harness image").expect("gpu frame")
}

fn inked(frame: &image::RgbaImage, x: u32, y: u32) -> bool {
    frame.get_pixel(x, y).0[0] > 16
}

#[test]
fn open_fill_subpath_closes_back_to_its_start() {
    let shown = frame(r##"<path d="M4 4H20V20H4" fill="#fff"/>"##);
    assert!(inked(&shown, 16, 16), "the fill reaches the implied closing edge");
    assert!(!inked(&shown, 2, 16), "nothing draws left of the closing edge");
}

#[test]
fn stroked_polyline_leaves_rows_outside_its_outline_empty() {
    let shown = frame(
        r##"<path d="M22 12h-2.48l-2.35 8.36L9.24 2.18l-2.35 8.36H2" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>"##,
    );
    let edge = (0..SIDE).filter(|y| inked(&shown, 0, *y)).count();
    let top = (0..SIDE).filter(|x| inked(&shown, *x, 0)).count();
    assert_eq!(edge, 0, "the left edge stays empty");
    assert_eq!(top, 0, "the top row stays empty");
    assert!(shown.pixels().any(|pixel| pixel.0[0] > 200), "the stroke draws");
}
