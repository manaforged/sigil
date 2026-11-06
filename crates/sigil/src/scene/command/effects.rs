use std::sync::Arc;

use crate::geometry::{Point, Rect};

use crate::scene::corner_radii::CornerRadii;
use crate::scene::shadow::Shadow;

use super::transform3d::Transform3D;
use super::{CommandScene, SceneCommand};

/// A 2D affine transform as two rows of three values.
pub type Transform = [[f32; 3]; 2];

/// How a renderer colors a sprite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpriteKind {
    /// The mono sprite kind.
    Mono,
    /// The subpixel sprite kind.
    Subpixel,
    /// The poly sprite kind.
    Poly,
}

/// Where a sprite's pixels come from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteSource {
    /// The atlas.
    pub atlas: u32,
    /// The region.
    pub region: Rect,
    /// The phase.
    pub phase: f32,
}

impl SpriteSource {
    #[cfg(all(
        feature = "scene",
        any(test, feature = "kittest", feature = "desktop", feature = "terminal")
    ))]
    pub(crate) fn fitted(self, atlas: (f32, f32), dest: Rect) -> Option<(Rect, Rect)> {
        let region = self.region;
        if region.width <= 0.0 || region.height <= 0.0 {
            return None;
        }
        let clipped = region.intersection(Rect::new(Point::new(0.0, 0.0), atlas.0, atlas.1))?;
        let sx = dest.width / region.width;
        let sy = dest.height / region.height;
        let visible = Rect::new(
            Point::new(
                (clipped.origin.x - region.origin.x).mul_add(sx, dest.origin.x),
                (clipped.origin.y - region.origin.y).mul_add(sy, dest.origin.y),
            ),
            clipped.width * sx,
            clipped.height * sy,
        );
        Some((clipped, visible))
    }
}

/// The line style of a text decoration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecorationKind {
    /// The underline decoration kind.
    Underline,
    /// The strikethrough decoration kind.
    Strikethrough,
    /// The wavy underline decoration kind.
    WavyUnderline,
}

/// The bounds and effects of a composite layer.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerScope {
    /// The clip.
    pub clip: Rect,
    /// The clip radii.
    pub clip_radii: CornerRadii,
    /// The effect.
    pub effect: LayerEffect,
    /// The opacity.
    pub opacity: f32,
    /// How the layer's pixels mix with the pixels behind it, as in CSS `mix-blend-mode`.
    pub blend: BlendMode,
}

impl LayerScope {
    pub(crate) const fn srgb(clip: Rect) -> Self {
        Self {
            clip,
            clip_radii: CornerRadii::ZERO,
            effect: LayerEffect::Identity,
            opacity: 1.0,
            blend: BlendMode::SrgbOver,
        }
    }
}

/// A CSS `mix-blend-mode`: the formula that mixes a layer's colors with its backdrop, in sRGB-encoded space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BlendMode {
    /// The source color replaces the backdrop color.
    #[default]
    Normal,
    /// Source times backdrop.
    Multiply,
    /// The inverse of the product of the inverses.
    Screen,
    /// Multiply or screen, chosen by the backdrop.
    Overlay,
    /// The darker of source and backdrop.
    Darken,
    /// The lighter of source and backdrop.
    Lighten,
    /// Brightens the backdrop toward the source.
    ColorDodge,
    /// Darkens the backdrop toward the source.
    ColorBurn,
    /// Multiply or screen, chosen by the source.
    HardLight,
    /// Darkens or lightens, chosen by the source, like a diffused spotlight.
    SoftLight,
    /// The absolute difference of source and backdrop.
    Difference,
    /// Like difference with lower contrast.
    Exclusion,
    /// The source's hue with the backdrop's saturation and luminosity.
    Hue,
    /// The source's saturation with the backdrop's hue and luminosity.
    Saturation,
    /// The source's hue and saturation with the backdrop's luminosity.
    Color,
    /// The source's luminosity with the backdrop's hue and saturation.
    Luminosity,
    #[doc(hidden)]
    SrgbOver,
}

impl BlendMode {
    /// Every mode in CSS Compositing and Blending order.
    pub const ALL: [Self; 16] = [
        Self::Normal,
        Self::Multiply,
        Self::Screen,
        Self::Overlay,
        Self::Darken,
        Self::Lighten,
        Self::ColorDodge,
        Self::ColorBurn,
        Self::HardLight,
        Self::SoftLight,
        Self::Difference,
        Self::Exclusion,
        Self::Hue,
        Self::Saturation,
        Self::Color,
        Self::Luminosity,
    ];
}

/// An effect applied to a composite layer.
#[derive(Clone, Debug, PartialEq)]
pub enum LayerEffect {
    /// The identity layer effect.
    Identity,
    /// The transform layer effect.
    Transform(Transform),
    /// The layer is drawn as a CSS 3D plane through a perspective transform.
    Projected {
        /// The full transform from layer space to the parent's space.
        transform: Transform3D,
        /// Draws nothing when the transform shows the layer's back, like `backface-visibility: hidden`.
        hide_backface: bool,
    },
    /// The layer's pixels are scaled by a mask, as in CSS `mask-image`.
    Mask(Mask),
    /// The layer's pixels pass through each filter in order, as in CSS `filter`.
    Filter(Arc<[Filter]>),
}

/// One CSS filter function, applied in sRGB as Filter Effects 1 §13 defines it.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Filter {
    /// A Gaussian blur whose standard deviation is this many logical pixels.
    Blur(f32),
    /// The layer's alpha, blurred by `blur` as a standard deviation, offset, tinted by `color`, drawn under the layer; `spread` is ignored.
    DropShadow(Shadow),
    /// Multiplies each color channel by the amount.
    Brightness(f32),
    /// Scales each color channel away from mid-gray by the amount.
    Contrast(f32),
    /// Mixes toward the luminance gray by the amount, clamped to 1.
    Grayscale(f32),
    /// Rotates the hue by this many degrees.
    HueRotate(f32),
    /// Mixes toward the inverted color by the amount, clamped to 1.
    Invert(f32),
    /// Multiplies alpha by the amount, clamped to 1.
    Opacity(f32),
    /// Scales saturation by the amount.
    Saturate(f32),
    /// Mixes toward sepia by the amount, clamped to 1.
    Sepia(f32),
}

impl Filter {
    pub(crate) const fn is_color(self) -> bool {
        !matches!(self, Self::Blur(_) | Self::DropShadow(_))
    }
}

pub(crate) fn filter_reach(filters: &[Filter]) -> f32 {
    filters
        .iter()
        .map(|filter| match *filter {
            Filter::Blur(sigma) => sigma.max(0.0) * 3.0,
            Filter::DropShadow(shadow) => {
                let [x, y] = shadow.offset;
                shadow.blur.max(0.0).mul_add(3.0, x.abs().max(y.abs()))
            }
            _ => 0.0,
        })
        .sum()
}

/// Which channel of a mask scales the masked pixels, as in CSS `mask-mode`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MaskMode {
    /// The mask's alpha.
    #[default]
    Alpha,
    /// The mask's luminance times its alpha.
    Luminance,
}

/// Drawing commands whose alpha or luminance scales a layer's pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct Mask {
    pub(crate) mode: MaskMode,
    pub(crate) commands: Arc<[SceneCommand]>,
}

impl Mask {
    /// A mask read through `mode` from the pixels `scene` draws, in the coordinates of the layer's children.
    #[must_use]
    pub fn new(mode: MaskMode, scene: CommandScene) -> Self {
        Self {
            mode,
            commands: scene.commands.into(),
        }
    }
}

pub(crate) fn transform_rect(transform: Transform, rect: Rect) -> Rect {
    let (left, top) = (rect.origin.x, rect.origin.y);
    let (right, bottom) = (left + rect.width, top + rect.height);
    let points = [(left, top), (right, top), (left, bottom), (right, bottom)]
        .map(|(x, y)| transform_point(transform, Point::new(x, y)));
    let fold = |axis: fn(&Point) -> f32, start: f32, pick: fn(f32, f32) -> f32| {
        points.iter().map(axis).fold(start, pick)
    };
    let (min_x, max_x) = (
        fold(|p| p.x, f32::INFINITY, f32::min),
        fold(|p| p.x, f32::NEG_INFINITY, f32::max),
    );
    let (min_y, max_y) = (
        fold(|p| p.y, f32::INFINITY, f32::min),
        fold(|p| p.y, f32::NEG_INFINITY, f32::max),
    );
    Rect::new(Point::new(min_x, min_y), max_x - min_x, max_y - min_y)
}

#[allow(
    clippy::missing_const_for_fn,
    reason = "const f32::mul_add requires Rust 1.94; Sigil supports 1.90"
)]
pub(crate) fn transform_point(transform: Transform, point: Point) -> Point {
    Point::new(
        transform[0][0].mul_add(point.x, transform[0][1].mul_add(point.y, transform[0][2])),
        transform[1][0].mul_add(point.x, transform[1][1].mul_add(point.y, transform[1][2])),
    )
}
