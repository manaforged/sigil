#![allow(dead_code)]

use sigil::hal::{Point, Rect};
use sigil::style::Rgba8;

pub(crate) const fn color(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

pub(crate) const fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::new(Point::new(x, y), width, height)
}
