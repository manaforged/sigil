/// Size in device pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhysicalSize {
    /// The width.
    pub width: u32,
    /// The height.
    pub height: u32,
}

impl PhysicalSize {
    /// Creates a physical size.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// A position in logical px.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// The x.
    pub x: f32,
    /// The y.
    pub y: f32,
}

impl Point {
    /// Creates a point.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// A rectangle in logical units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// The origin.
    pub origin: Point,
    /// The width.
    pub width: f32,
    /// The height.
    pub height: f32,
}

impl Rect {
    /// Creates a rect.
    #[must_use]
    pub const fn new(origin: Point, width: f32, height: f32) -> Self {
        Self {
            origin,
            width,
            height,
        }
    }

    /// Returns whether the two rectangles overlap.
    #[must_use]
    pub fn intersects(self, other: Self) -> bool {
        self.origin.x < other.origin.x + other.width
            && other.origin.x < self.origin.x + self.width
            && self.origin.y < other.origin.y + other.height
            && other.origin.y < self.origin.y + self.height
    }

    /// Returns the overlap of the two rectangles, if any.
    #[must_use]
    pub fn intersection(self, other: Self) -> Option<Self> {
        let x = self.origin.x.max(other.origin.x);
        let y = self.origin.y.max(other.origin.y);
        let right = (self.origin.x + self.width).min(other.origin.x + other.width);
        let bottom = (self.origin.y + self.height).min(other.origin.y + other.height);
        let width = right - x;
        let height = bottom - y;
        if width <= 0.0 || height <= 0.0 {
            None
        } else {
            Some(Self::new(Point::new(x, y), width, height))
        }
    }

    /// Returns the smallest rectangle that holds both rectangles.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        let x = self.origin.x.min(other.origin.x);
        let y = self.origin.y.min(other.origin.y);
        let right = (self.origin.x + self.width).max(other.origin.x + other.width);
        let bottom = (self.origin.y + self.height).max(other.origin.y + other.height);
        Self::new(Point::new(x, y), right - x, bottom - y)
    }

    /// Returns this rectangle grown on each side.
    #[must_use]
    pub const fn expand(self, pad: f32) -> Self {
        Self::new(
            Point::new(self.origin.x - pad, self.origin.y - pad),
            self.width + pad * 2.0,
            self.height + pad * 2.0,
        )
    }
}

/// A color with `f32` red, green, blue, and alpha channels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    /// The r.
    pub r: f32,
    /// The g.
    pub g: f32,
    /// The b.
    pub b: f32,
    /// The a.
    pub a: f32,
}

impl Default for Rgba {
    fn default() -> Self {
        Self::TRANSPARENT
    }
}

impl Rgba {
    /// Fully transparent black.
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    /// Opaque black.
    #[cfg(any(test, feature = "kittest"))]
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    /// Opaque white.
    #[cfg(any(test, feature = "kittest", feature = "scene"))]
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };

    /// Creates a color from its four channels.
    #[must_use]
    #[cfg(any(test, feature = "kittest", feature = "scene"))]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}
