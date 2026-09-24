use crate::hal::{Point, Rect};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ShadowKind {
    #[default]
    Outer,
    Inner,
    Glow,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub color: [f32; 4],
    pub offset: [f32; 2],
    pub blur: f32,
    pub spread: f32,
    pub kind: ShadowKind,
}

impl Shadow {
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

    #[must_use]
    pub fn glow(color: crate::style::Rgba8, spread: f32, blur: f32) -> Self {
        let c = [
            srgb_channel_to_linear(color.r),
            srgb_channel_to_linear(color.g),
            srgb_channel_to_linear(color.b),
            f32::from(color.a) / 255.0,
        ];
        Self::new([0.0, 0.0], blur, spread, c).with_kind(ShadowKind::Glow)
    }

    #[must_use]
    pub const fn with_kind(mut self, kind: ShadowKind) -> Self {
        self.kind = kind;
        self
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

pub(crate) fn srgb_channel_to_linear(channel: u8) -> f32 {
    let value = f32::from(channel) / 255.0;
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}
