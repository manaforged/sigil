use image::{Rgba, RgbaImage};
use sigil::prelude::*;
use sigil_kittest::Harness;

const WIDTH: u32 = 360;
const HEIGHT: u32 = 160;
const BOX_DELTA: i32 = 6;
const INK_DELTA: i32 = 120;

pub(crate) struct Ink {
    pub(crate) box_top: u32,
    pub(crate) box_bottom: u32,
    pub(crate) top: u32,
    pub(crate) bottom: u32,
    pub(crate) rows: u32,
}

fn delta(a: &Rgba<u8>, b: &Rgba<u8>) -> i32 {
    (0..3)
        .map(|channel| (i32::from(a[channel]) - i32::from(b[channel])).abs())
        .sum()
}

pub(crate) fn page(child: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().padding(16.0))
        .bg(Token::Surface)
        .child(child)
}

pub(crate) fn frame(build: impl IntoNode + 'static) -> RgbaImage {
    Harness::new(build)
        .pixels(WIDTH, HEIGHT)
        .image()
        .expect("render")
        .expect("a frame")
}

fn row_has(image: &RgbaImage, y: u32, xs: (u32, u32), bg: &Rgba<u8>, by: i32) -> bool {
    (xs.0..xs.1).any(|x| delta(image.get_pixel(x, y), bg) > by)
}

pub(crate) fn ink(image: &RgbaImage) -> Ink {
    let page = *image.get_pixel(WIDTH - 1, HEIGHT - 1);
    let boxed: Vec<u32> = (0..HEIGHT)
        .filter(|y| row_has(image, *y, (0, WIDTH), &page, BOX_DELTA))
        .collect();
    let (box_top, box_bottom) = (boxed[0], boxed[boxed.len() - 1]);
    let centre = (box_top + box_bottom) / 2;
    let left = (0..WIDTH)
        .find(|x| delta(image.get_pixel(*x, centre), &page) > BOX_DELTA)
        .unwrap_or(0);
    let right = (0..WIDTH)
        .rev()
        .find(|x| delta(image.get_pixel(*x, centre), &page) > BOX_DELTA)
        .unwrap_or(WIDTH - 1);
    let inner = *image.get_pixel(right.saturating_sub(4), centre);
    let xs = (left + 6, right.saturating_sub(4).max(left + 7));
    let rows: Vec<u32> = (box_top..=box_bottom)
        .filter(|y| {
            let at_edge = *y < box_top + 3 || *y + 3 > box_bottom;
            !(at_edge && right > left + 40) && row_has(image, *y, xs, &inner, INK_DELTA)
        })
        .collect();
    Ink {
        box_top,
        box_bottom,
        top: rows.first().copied().unwrap_or(box_top),
        bottom: rows.last().copied().unwrap_or(box_bottom),
        rows: rows.len() as u32,
    }
}

pub(crate) fn shadow_rows(image: &RgbaImage) -> u32 {
    let page = *image.get_pixel(WIDTH - 1, HEIGHT - 1);
    let middle = WIDTH / 2;
    let solid = (0..HEIGHT).rev().find(|y| delta(image.get_pixel(middle, *y), &page) > 24);
    let Some(solid) = solid else {
        return (0..HEIGHT)
            .filter(|y| delta(image.get_pixel(middle, *y), image.get_pixel(0, 0)) > 0)
            .count() as u32;
    };
    (solid + 1..HEIGHT)
        .filter(|y| delta(image.get_pixel(middle, *y), &page) > 0)
        .count() as u32
}
