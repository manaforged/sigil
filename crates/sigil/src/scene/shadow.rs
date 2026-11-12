use crate::geometry::{Point, Rect};
use crate::style::srgb;

kittest_pub! {
    /// Whether a shadow falls outside or inside a shape.
    #[doc(hidden)]
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub enum ShadowKind {
        /// The outer shadow kind.
        #[default]
        Outer,
        /// The inner shadow kind.
        Inner,
        /// The glow shadow kind.
        Glow,
    }
}

/// The offset, blur, spread, and color of a shadow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    /// The color.
    pub color: [f32; 4],
    /// The offset.
    pub offset: [f32; 2],
    /// The blur.
    pub blur: f32,
    /// The spread.
    pub spread: f32,
    #[cfg(feature = "kittest")]
    #[doc(hidden)]
    pub kind: ShadowKind,
    #[cfg(not(feature = "kittest"))]
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

    /// Creates a glow: a shadow of `color` with no offset.
    #[must_use]
    pub fn glow(color: crate::style::Color, spread: f32, blur: f32) -> Self {
        let [r, g, b] = srgb::decode_rgb(color);
        let c = [r, g, b, srgb::unit(color.a)];
        Self::new([0.0, 0.0], blur, spread, c).kind(ShadowKind::Glow)
    }

    kittest_pub! {
        /// Sets whether the shadow is outer, inner, or a glow.
        #[must_use]
        #[doc(hidden)]
        pub const fn kind(mut self, kind: ShadowKind) -> Self {
            self.kind = kind;
            self
        }
    }
}

pub(crate) fn expanded_shadow_rect(surface: Rect, shadow: Shadow) -> Rect {
    if shadow.kind == ShadowKind::Inner {
        return surface;
    }
    let [offset_x, offset_y] = shadow.offset;
    let outset = shadow.blur.mul_add(3.0, shadow.spread.abs());
    Rect::new(
        Point::new(
            surface.origin.x - outset + offset_x,
            surface.origin.y - outset + offset_y,
        ),
        surface.width + outset * 2.0,
        surface.height + outset * 2.0,
    )
}
