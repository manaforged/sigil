#![allow(dead_code)]

use sigil::geometry::{Point, Rect};
use sigil::style::Color;

pub(crate) const fn color(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color { r, g, b, a }
}

pub(crate) const fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::new(Point::new(x, y), width, height)
}
