//! Layer effect and sprite kind types used by [`SceneCommand`](super::SceneCommand).

use hal::Rect;

/// Row-major 2D affine transform.
///
/// The two rows encode the transform from local coordinates to parent
/// coordinates as `x' = ax + cy + tx`, `y' = bx + dy + ty`.
pub type Transform = [[f32; 3]; 2];

/// Which rendering path to use for a sprite draw command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpriteKind {
    /// Alpha coverage in the texture's A channel, tinted by `color`.
    Mono,
    /// RGB subpixel coverage (R/G/B channels carry phase-offset coverage).
    Subpixel,
    /// Full RGBA atlas color; the `color` tint field is ignored.
    Poly,
}

/// Source region within an atlas texture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteSource {
    /// Atlas id returned by `SpritePipeline::register_atlas`.
    pub atlas: u32,
    /// Region within the atlas, in atlas-space pixels.
    pub region: Rect,
    /// Subpixel-x phase offset in `[0.0, 1.0)` for `SpriteKind::Subpixel`.
    pub phase: f32,
}

/// Which text decoration to render.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecorationKind {
    /// Solid horizontal underline.
    Underline,
    /// Solid horizontal strikethrough.
    Strikethrough,
    /// Wavy (sinusoidal) underline, e.g. for spell-check.
    WavyUnderline,
}

/// Clip region and compositing parameters for an offscreen layer.
///
/// Used with [`SceneCommand::PushLayer`](super::SceneCommand::PushLayer) /
/// [`SceneCommand::PopLayer`](super::SceneCommand::PopLayer) to
/// delimit a subtree of draw commands that renders into an offscreen texture
/// and is then composited back with the given effect and opacity.
///
/// Maximum nesting depth is 8. In debug builds a depth violation panics with
/// a clear message; in release builds the excess layer is silently dropped.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerScope {
    /// Logical-pixel clip rectangle for this layer.
    pub clip: Rect,
    /// Effect applied to the layer content before compositing.
    pub effect: LayerEffect,
    /// Layer opacity multiplier in `[0.0, 1.0]`.
    pub opacity: f32,
}

/// Effect applied to a layer texture before compositing back to the parent.
///
/// `Identity` passes the layer content through unchanged. `LayerBlur`
/// applies a dual-Kawase blur at the given effective sigma (in logical pixels)
/// before compositing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LayerEffect {
    /// No effect — pass-through. The layer is composited at `LayerScope::opacity`.
    Identity,
    /// Dual-Kawase blur applied to the layer's own composited content before
    /// compositing back to the parent target.
    ///
    /// This blurs the layer's own content — not the parent backdrop behind it.
    /// For CSS-style backdrop-filter (parent buffer snapshot + blur-under composite)
    /// see DR-035 (deferred to R15).
    ///
    /// `sigma == 0.0` behaves identically to `Identity`. Sigmas above 32 are clamped
    /// to 3 pyramid passes (same as `sigma = 32`).
    LayerBlur {
        /// Effective blur radius in logical pixels.
        sigma: f32,
    },
}
