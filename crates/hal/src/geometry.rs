//! Shared geometric and colour primitives used across HAL traits.
//!
//! These types are deliberately small and `Copy`. Richer types (styled
//! colours, layout boxes, affine transforms) live in higher crates and
//! convert to these at the HAL boundary.

/// A size in physical device pixels.
///
/// Distinct from logical size — scale factor conversion happens above
/// the HAL. Backends receive and return physical pixels only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhysicalSize {
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
}

impl PhysicalSize {
    /// Construct a physical size.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// A 2D point in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// X coordinate.
    pub x: f32,
    /// Y coordinate.
    pub y: f32,
}

impl Point {
    /// Construct a point.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// An axis-aligned rectangle in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Top-left corner.
    pub origin: Point,
    /// Width in logical pixels.
    pub width: f32,
    /// Height in logical pixels.
    pub height: f32,
}

impl Rect {
    /// Construct a rectangle.
    #[must_use]
    pub const fn new(origin: Point, width: f32, height: f32) -> Self {
        Self {
            origin,
            width,
            height,
        }
    }
}

/// Linear-space RGBA colour, each channel in `[0.0, 1.0]`.
///
/// The HAL takes linear colour. The theme layer converts from sRGB or
/// OKLCH before calling into the HAL.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// Red channel, linear.
    pub r: f32,
    /// Green channel, linear.
    pub g: f32,
    /// Blue channel, linear.
    pub b: f32,
    /// Alpha channel.
    pub a: f32,
}

impl Default for Color {
    /// Default colour is transparent black (all channels zero).
    fn default() -> Self {
        Self::TRANSPARENT
    }
}

impl Color {
    /// Transparent black.
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    /// Opaque black.
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    /// Opaque white.
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };

    /// Construct a linear RGBA colour.
    #[must_use]
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}
