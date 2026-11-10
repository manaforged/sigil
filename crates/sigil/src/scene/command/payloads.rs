#![allow(clippy::redundant_pub_crate)]

mod marks;
mod path;

pub use marks::{DrawDecoration, DrawSprite};
pub use path::DrawPath;
#[cfg(any(feature = "scene", feature = "terminal"))]
pub(crate) use path::MITER_LIMIT;
pub(crate) use path::{PathCmd, PathStroke, outline_key, rounded_outline};

use crate::geometry::Rect;
use crate::kernel::NodeId;
use crate::style::{Color, Tone};

use crate::scene::corner_radii::CornerRadii;

use super::fills::{PatternKind, Stroke};
use super::gradient::Gradient;

/// A command that fills or strokes a rounded rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawRect {
    /// The node, if any.
    pub node: Option<NodeId>,
    /// The rect.
    pub rect: Rect,
    /// The color.
    pub color: Color,
    #[cfg(feature = "kittest")]
    #[doc(hidden)]
    pub border_radius: CornerRadii,
    #[cfg(not(feature = "kittest"))]
    pub(crate) border_radius: CornerRadii,
    /// The opacity.
    pub opacity: u16,
    #[cfg(feature = "kittest")]
    #[doc(hidden)]
    pub stroke: Option<Stroke>,
    #[cfg(not(feature = "kittest"))]
    pub(crate) stroke: Option<Stroke>,
    /// The tone, if any.
    pub tone: Option<Tone>,
    /// Whether leaf.
    pub leaf: bool,
    #[cfg(feature = "kittest")]
    #[doc(hidden)]
    pub border: Option<CellBorder>,
    #[cfg(not(feature = "kittest"))]
    pub(crate) border: Option<CellBorder>,
    /// Whether outline.
    pub outline: bool,
}

/// A box-drawing border on terminal cells.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellBorder {
    /// The rect.
    pub rect: Rect,
    /// The widths.
    pub widths: [f32; 4],
    /// The color.
    pub color: Color,
    /// Whether rounded.
    pub rounded: bool,
}

/// A rectangle aligned to terminal cells.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct CellRect {
    /// The tone, if any.
    pub tone: Option<Tone>,
    /// Whether leaf.
    pub leaf: bool,
    /// The border, if any.
    pub border: Option<CellBorder>,
    /// Whether outline.
    pub outline: bool,
}

impl DrawRect {
    /// A solid rectangle with square corners at full opacity.
    #[must_use]
    pub const fn new(rect: Rect, color: Color) -> Self {
        Self {
            node: None,
            rect,
            color,
            border_radius: CornerRadii::ZERO,
            opacity: 10_000,
            stroke: None,
            tone: None,
            leaf: false,
            border: None,
            outline: false,
        }
    }

    kittest_pub! {
    /// Sets the corner radii.
    #[must_use]
    #[doc(hidden)]
    pub const fn border_radius(mut self, border_radius: CornerRadii) -> Self {
        self.border_radius = border_radius;
        self
    }
    }

    /// Sets the opacity in basis points; 10000 is opaque.
    #[must_use]
    pub const fn opacity(mut self, opacity: u16) -> Self {
        self.opacity = opacity;
        self
    }

    kittest_pub! {
        /// Sets the stroke.
        #[must_use]
        #[doc(hidden)]
        pub const fn stroke(mut self, stroke: Stroke) -> Self {
            self.stroke = Some(stroke);
            self
        }
    }

    pub(crate) const fn with_node(mut self, node: NodeId) -> Self {
        self.node = Some(node);
        self
    }

    pub(crate) const fn with_cell(mut self, cell: CellRect) -> Self {
        self.tone = cell.tone;
        self.leaf = cell.leaf;
        self.border = cell.border;
        self.outline = cell.outline;
        self
    }
}

/// A command that fills a shape with a gradient.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawGradient {
    /// The node, if any.
    pub node: Option<NodeId>,
    /// The rect.
    pub rect: Rect,
    /// The gradient.
    pub gradient: Gradient,
    /// The border radius.
    pub border_radius: CornerRadii,
    /// The opacity.
    pub opacity: u16,
    /// The stroke, if any.
    pub stroke: Option<Stroke>,
    /// The tone, if any.
    pub tone: Option<Tone>,
    /// Whether leaf.
    pub leaf: bool,
    /// The border, if any.
    pub border: Option<CellBorder>,
    /// Whether outline.
    pub outline: bool,
}

impl DrawGradient {
    /// A gradient rectangle with square corners at full opacity.
    #[must_use]
    pub fn new(rect: Rect, gradient: impl Into<Gradient>) -> Self {
        Self {
            node: None,
            rect,
            gradient: gradient.into(),
            border_radius: CornerRadii::ZERO,
            opacity: 10_000,
            stroke: None,
            tone: None,
            leaf: false,
            border: None,
            outline: false,
        }
    }

    /// Sets the corner radii.
    #[must_use]
    pub const fn border_radius(mut self, border_radius: CornerRadii) -> Self {
        self.border_radius = border_radius;
        self
    }

    /// Sets the opacity in basis points; 10000 is opaque.
    #[must_use]
    pub const fn opacity(mut self, opacity: u16) -> Self {
        self.opacity = opacity;
        self
    }

    /// Sets the stroke.
    #[must_use]
    pub const fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }

    pub(crate) const fn with_node(mut self, node: NodeId) -> Self {
        self.node = Some(node);
        self
    }

    pub(crate) const fn with_cell(mut self, cell: CellRect) -> Self {
        self.tone = cell.tone;
        self.leaf = cell.leaf;
        self.border = cell.border;
        self.outline = cell.outline;
        self
    }
}

/// A command that fills a shape with a pattern.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawPattern {
    /// The node, if any.
    pub node: Option<NodeId>,
    /// The rect.
    pub rect: Rect,
    /// The pattern.
    pub pattern: PatternKind,
    /// The border radius.
    pub border_radius: CornerRadii,
    /// The opacity.
    pub opacity: u16,
    /// The stroke, if any.
    pub stroke: Option<Stroke>,
}

impl DrawPattern {
    /// A pattern rectangle with square corners at full opacity.
    #[must_use]
    pub const fn new(rect: Rect, pattern: PatternKind) -> Self {
        Self {
            node: None,
            rect,
            pattern,
            border_radius: CornerRadii::ZERO,
            opacity: 10_000,
            stroke: None,
        }
    }

    /// Sets the corner radii.
    #[must_use]
    pub const fn border_radius(mut self, border_radius: CornerRadii) -> Self {
        self.border_radius = border_radius;
        self
    }

    /// Sets the opacity in basis points; 10000 is opaque.
    #[must_use]
    pub const fn opacity(mut self, opacity: u16) -> Self {
        self.opacity = opacity;
        self
    }

    /// Sets the stroke.
    #[must_use]
    pub const fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }
}

/// An image command fitted into a destination rectangle, with optional crop.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawImage {
    /// The node, if any.
    pub node: Option<NodeId>,
    /// The source.
    pub source: crate::kernel::ImageSource,
    /// The fit.
    pub fit: crate::kernel::ObjectFit,
    /// The destination.
    pub destination: Rect,
    /// The opacity.
    pub opacity: u16,
    /// The crop, if any.
    pub crop: Option<crate::kernel::ImageCrop>,
    /// The raster, if any.
    pub raster: Option<crate::kernel::ImageRaster>,
}

const RASTER_MAX_SIDE: f32 = 4096.0;

impl DrawImage {
    /// The image and destination to paint at `scale`.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "the extent is clamped to a bounded positive range before the cast"
    )]
    #[must_use]
    pub fn source_at(&self, scale: f32) -> (crate::kernel::ImageSource, Rect) {
        let dest = self.destination;
        let raster = self
            .raster
            .as_ref()
            .filter(|_| self.crop.is_none() && scale.is_finite() && scale > 0.0);
        let Some(raster) = raster else {
            return (self.source.clone(), dest);
        };
        let left = (dest.origin.x * scale).round();
        let top = (dest.origin.y * scale).round();
        let width = ((dest.origin.x + dest.width) * scale).round() - left;
        let height = ((dest.origin.y + dest.height) * scale).round() - top;
        let side = |extent: f32| extent.clamp(1.0, RASTER_MAX_SIDE) as u32;
        let Some(source) = raster.at(side(width), side(height), scale) else {
            return (self.source.clone(), dest);
        };
        let snapped = Rect::new(
            crate::geometry::Point::new(left / scale, top / scale),
            source.width() as f32 / scale,
            source.height() as f32 / scale,
        );
        (source, snapped)
    }
}
