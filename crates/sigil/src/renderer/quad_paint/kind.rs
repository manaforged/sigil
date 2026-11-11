use bytemuck::{Pod, Zeroable};

use crate::renderer::scene_command::{SpriteKind, Transform};
use crate::renderer::scene_instance::affine_rows;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum QuadKind {
    #[cfg(feature = "kittest")]
    Mono,
    #[cfg(feature = "kittest")]
    Subpixel,
    Poly,
    Mask,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum QuadBlend {
    Straight,
    Premultiplied,
}

impl QuadKind {
    const fn code(self) -> f32 {
        match self {
            #[cfg(feature = "kittest")]
            Self::Mono => 0.0,
            #[cfg(feature = "kittest")]
            Self::Subpixel => 1.0,
            Self::Poly => 2.0,
            Self::Mask => 3.0,
        }
    }

    pub(super) const fn blend(self) -> QuadBlend {
        if matches!(self, Self::Mask) {
            QuadBlend::Straight
        } else {
            QuadBlend::Premultiplied
        }
    }
}

impl From<SpriteKind> for QuadKind {
    fn from(kind: SpriteKind) -> Self {
        match kind {
            #[cfg(feature = "kittest")]
            SpriteKind::Mono => Self::Mono,
            #[cfg(feature = "kittest")]
            SpriteKind::Subpixel => Self::Subpixel,
            SpriteKind::Poly => Self::Poly,
        }
    }
}

impl QuadBlend {
    pub(super) const fn state(self) -> wgpu::BlendState {
        match self {
            Self::Straight => wgpu::BlendState::ALPHA_BLENDING,
            Self::Premultiplied => wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING,
        }
    }

    pub(super) const fn pass_label(self) -> &'static str {
        match self {
            Self::Straight => "raster-text-pass",
            Self::Premultiplied => "sigil-renderer sprite pass",
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct QuadInstance {
    dest: [f32; 4],
    pub(crate) uv_rect: [f32; 4],
    color: [f32; 4],
    params: [f32; 4],
    xform: [[f32; 4]; 2],
}

impl QuadInstance {
    pub(crate) const fn new(
        (dest, uv_rect): ([f32; 4], [f32; 4]),
        color: [f32; 4],
        (kind, opacity, phase): (QuadKind, f32, f32),
        transform: Transform,
    ) -> Self {
        Self {
            dest,
            uv_rect,
            color,
            params: [opacity, kind.code(), phase, 0.0],
            xform: affine_rows(transform),
        }
    }
}
