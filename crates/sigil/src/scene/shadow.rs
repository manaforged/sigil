use crate::geometry::{Point, Rect};
use crate::style::{Color, Elevation, srgb};

/// Whether a shadow falls outside or inside a shape.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ShadowKind {
    /// Falls outside the shape, moved by the offset and grown by the spread.
    #[default]
    Outer,
    /// Falls inside the shape's edge, where a positive spread makes it reach further in.
    Inner,
    /// A halo around the shape, painted like `Outer`; `Shadow::glow` makes one with no offset.
    Glow,
}

/// The offset, blur, spread, and color of a shadow.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Shadow {
    pub(crate) color: [f32; 4],
    pub(crate) offset: [f32; 2],
    pub(crate) blur: f32,
    pub(crate) spread: f32,
    pub(crate) kind: ShadowKind,
}

impl Shadow {
    /// Creates an outer shadow from its offset, blur, spread, and RGBA color.
    #[must_use]
    pub const fn new(offset: [f32; 2], blur: f32, spread: f32, color: [f32; 4]) -> Self {
        Self {
            color,
            offset,
            blur,
            spread,
            kind: ShadowKind::Outer,
        }
    }

    /// Red, green, and blue in linear light, then alpha, each from 0 to 1 and not premultiplied.
    #[must_use]
    pub const fn color(&self) -> [f32; 4] {
        self.color
    }

    /// How far the shadow moves right and down from the shape, in logical pixels.
    #[must_use]
    pub const fn offset(&self) -> [f32; 2] {
        self.offset
    }

    /// The standard deviation of the blur, in logical pixels; a negative value counts as 0.
    #[must_use]
    pub const fn blur(&self) -> f32 {
        self.blur
    }

    /// How far the shadow grows past the shape's edge before the blur, in logical pixels; a negative value shrinks it.
    #[must_use]
    pub const fn spread(&self) -> f32 {
        self.spread
    }

    /// Creates a glow of `color` with no offset; `blur` and `spread` come in the order of `new`, and the halo reaches `3 * blur + spread` logical pixels past the shape.
    #[must_use]
    pub fn glow(color: Color, blur: f32, spread: f32) -> Self {
        let [r, g, b] = srgb::decode_rgb(color);
        let c = [r, g, b, srgb::unit(color.a)];
        Self::new([0.0, 0.0], blur, spread, c).kind(ShadowKind::Glow)
    }

    /// Sets whether the shadow is outer, inner, or a glow.
    #[must_use]
    pub const fn kind(mut self, kind: ShadowKind) -> Self {
        self.kind = kind;
        self
    }

    /// Returns whether the shadow is outer, inner, or a glow.
    #[must_use]
    pub const fn shadow_kind(&self) -> ShadowKind {
        self.kind
    }

    /// Returns how far the shadow paints past its shape before the offset, `3 * blur + |spread|` logical pixels.
    #[must_use]
    pub fn reach(&self) -> f32 {
        self.blur.mul_add(3.0, self.spread.abs())
    }
}

impl From<Elevation> for Shadow {
    fn from(elevation: Elevation) -> Self {
        elevation_shadow(elevation)
    }
}

pub(crate) const fn elevation_shadow(elevation: Elevation) -> Shadow {
    let (offset_y, blur, spread, alpha) = match elevation {
        Elevation::Sm => (1.0, 2.0, 0.0, 0.10),
        Elevation::Base => (2.0, 4.0, 0.0, 0.12),
        Elevation::Md => (4.0, 8.0, -1.0, 0.14),
        Elevation::Lg => (8.0, 16.0, -2.0, 0.16),
        Elevation::Xl => (16.0, 28.0, -4.0, 0.20),
    };
    Shadow::new([0.0, offset_y], blur, spread, [0.0, 0.0, 0.0, alpha])
}

pub(crate) fn expanded_shadow_rect(surface: Rect, shadow: Shadow) -> Rect {
    if shadow.kind == ShadowKind::Inner {
        return surface;
    }
    let [offset_x, offset_y] = shadow.offset;
    let outset = shadow.reach();
    Rect::new(
        Point::new(
            surface.origin.x - outset + offset_x,
            surface.origin.y - outset + offset_y,
        ),
        surface.width + outset * 2.0,
        surface.height + outset * 2.0,
    )
}
