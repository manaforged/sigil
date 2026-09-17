use crate::hal::Rect;

use crate::renderer::CornerRadii;

pub type Transform = [[f32; 3]; 2];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpriteKind {
    Mono,
    Subpixel,
    Poly,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteSource {
    pub atlas: u32,
    pub region: Rect,
    pub phase: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecorationKind {
    Underline,
    Strikethrough,
    WavyUnderline,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerScope {
    pub clip: Rect,
    pub clip_radii: CornerRadii,
    pub effect: LayerEffect,
    pub opacity: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LayerEffect {
    Identity,
    Transform(Transform),
    LayerBlur { sigma: f32 },
}
