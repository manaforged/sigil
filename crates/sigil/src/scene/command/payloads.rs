#![allow(clippy::redundant_pub_crate)]

mod marks;
mod path;

kittest_pub! {
    #[doc(hidden)]
    pub use marks::DrawDecoration;
}
kittest_pub! {
    #[doc(hidden)]
    #[cfg(scene_paint)]
    pub use marks::DrawSprite;
}
kittest_pub! {
    #[doc(hidden)]
    pub use path::DrawPath;
}
#[cfg(vector_paint)]
pub(crate) use path::MITER_LIMIT;
pub(crate) use path::{PathCmd, PathStroke, outline_key, rounded_outline};

use crate::geometry::Rect;
use crate::kernel::NodeId;
use crate::style::{Color, Tone};

use crate::scene::corner_radii::CornerRadii;

use super::fills::{PatternKind, Stroke};
use super::gradient::Gradient;

kittest_pub! {
    /// A command that fills or strokes a rounded rectangle.
    #[doc(hidden)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    #[non_exhaustive]
    pub struct DrawRect {
        /// The node this rectangle paints, or `None` when no node owns it.
        pub node: Option<NodeId>,
        /// The bounds of the rectangle, in logical pixels.
        pub rect: Rect,
        /// The fill color; a transparent color leaves only the stroke.
        pub color: Color,
        /// The radius of each corner.
        pub border_radius: CornerRadii,
        /// The opacity in basis points; 10000 is opaque.
        pub opacity: u16,
        /// The stroke around the rectangle, if any.
        pub stroke: Option<Stroke>,
        /// The theme tone the terminal host fills with when `color` is fully transparent.
        pub tone: Option<Tone>,
        /// True when the node has no laid-out child, so the terminal host may fill a partly covered cell with a half block.
        pub leaf: bool,
        /// The box-drawing border the terminal host draws around the node, if it has one.
        pub border: Option<CellBorder>,
        /// True for a focus ring or a border strip, which the terminal host does not fill.
        pub outline: bool,
    }
}

kittest_pub! {
    /// A box-drawing border on terminal cells.
    #[doc(hidden)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct CellBorder {
        /// The outer bounds of the bordered box, in logical pixels.
        pub(crate) rect: Rect,
        /// The widths of the top, right, bottom, and left edges; an edge of 0 or less is not drawn.
        pub(crate) widths: [f32; 4],
        /// The color of the box-drawing characters; a fully transparent color draws no border.
        pub(crate) color: Color,
        /// True when the box has a corner radius, so the terminal host draws rounded corner characters.
        pub(crate) rounded: bool,
    }
}

/// A rectangle aligned to terminal cells.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct CellRect {
    /// The theme tone the terminal host fills with when the command's color is fully transparent.
    pub tone: Option<Tone>,
    /// True when the node has no laid-out child, so the terminal host may fill a partly covered cell with a half block.
    pub leaf: bool,
    /// The box-drawing border the terminal host draws around the node, if it has one.
    pub border: Option<CellBorder>,
    /// True for a focus ring or a border strip, which the terminal host does not fill.
    pub outline: bool,
}

macro_rules! node_and_cell {
    ($($command:ty),+) => {$(
        impl $command {
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
    )+};
}

node_and_cell!(DrawRect, DrawGradient);

impl DrawRect {
    kittest_pub! {
    /// A solid rectangle with square corners at full opacity.
    #[doc(hidden)]
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
    }

    kittest_pub! {
    /// Sets the radius of each corner of the rectangle.
    #[must_use]
    #[doc(hidden)]
    pub const fn border_radius(mut self, border_radius: CornerRadii) -> Self {
        self.border_radius = border_radius;
        self
    }
    }

    kittest_pub! {
    /// Sets the opacity in basis points; 10000 is opaque.
    #[doc(hidden)]
    #[must_use]
    pub const fn opacity(mut self, opacity: u16) -> Self {
        self.opacity = opacity;
        self
    }
    }

    kittest_pub! {
        /// Adds a stroke to the rectangle.
        #[must_use]
        #[doc(hidden)]
        pub const fn stroke(mut self, stroke: Stroke) -> Self {
            self.stroke = Some(stroke);
            self
        }
    }
}

kittest_pub! {
    /// A command that fills a shape with a gradient.
    #[doc(hidden)]
    #[derive(Clone, Debug, PartialEq)]
    pub struct DrawGradient {
        /// The node this gradient paints, or `None` when no node owns it.
        pub node: Option<NodeId>,
        /// The bounds of the filled rectangle, in logical pixels.
        pub rect: Rect,
        /// The geometry and color stops of the fill.
        pub gradient: Gradient,
        /// The radius of each corner of the filled rectangle.
        pub border_radius: CornerRadii,
        /// The opacity in basis points; 10000 is opaque.
        pub opacity: u16,
        /// The line drawn along the edge of the rectangle, if it has one.
        pub stroke: Option<Stroke>,
        /// The theme tone the terminal host fills with when the gradient flattens to a fully transparent color.
        pub tone: Option<Tone>,
        /// True when the node has no laid-out child, so the terminal host may fill a partly covered cell with a half block.
        pub leaf: bool,
        /// The box-drawing border the terminal host draws around the node, if it has one.
        pub border: Option<CellBorder>,
        /// True for a focus ring or a border strip, which the terminal host does not fill.
        pub outline: bool,
    }
}

impl DrawGradient {
    kittest_pub! {
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
    }

    kittest_pub! {
        /// Sets the radius of each corner of the gradient fill.
        #[must_use]
        pub const fn border_radius(mut self, border_radius: CornerRadii) -> Self {
            self.border_radius = border_radius;
            self
        }
    }

    kittest_pub! {
        /// Sets the opacity in basis points; 10000 is opaque.
        #[must_use]
        pub const fn opacity(mut self, opacity: u16) -> Self {
            self.opacity = opacity;
            self
        }
    }

    kittest_pub! {
        /// Adds a stroke to the gradient fill.
        #[must_use]
        pub const fn stroke(mut self, stroke: Stroke) -> Self {
            self.stroke = Some(stroke);
            self
        }
    }
}

kittest_pub! {
    /// A command that fills a shape with a pattern.
    #[doc(hidden)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct DrawPattern {
        /// The node this pattern paints, or `None` when no node owns it.
        pub node: Option<NodeId>,
        /// The bounds of the filled rectangle, in logical pixels; the pattern repeats from its origin.
        pub rect: Rect,
        /// The shape and colors of the repeating fill.
        pub pattern: PatternKind,
        /// The radius of each corner of the filled rectangle.
        pub border_radius: CornerRadii,
        /// The opacity in basis points; 10000 is opaque.
        pub opacity: u16,
        /// The line drawn along the edge of the rectangle, if it has one.
        pub stroke: Option<Stroke>,
    }
}

impl DrawPattern {
    kittest_pub! {
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
    }

    kittest_pub! {
        if test;
        /// Sets the radius of each corner of the pattern fill.
        #[must_use]
        pub const fn border_radius(mut self, border_radius: CornerRadii) -> Self {
            self.border_radius = border_radius;
            self
        }
    }

    kittest_pub! {
        if test;
        /// Sets the opacity in basis points; 10000 is opaque.
        #[must_use]
        pub const fn opacity(mut self, opacity: u16) -> Self {
            self.opacity = opacity;
            self
        }
    }

    kittest_pub! {
        if test;
        /// Adds a stroke to the pattern fill.
        #[must_use]
        pub const fn stroke(mut self, stroke: Stroke) -> Self {
            self.stroke = Some(stroke);
            self
        }
    }
}

kittest_pub! {
    /// An image command fitted into a destination rectangle, with optional crop.
    #[doc(hidden)]
    #[derive(Clone, Debug, PartialEq)]
    pub struct DrawImage {
        /// The node this image paints, or `None` when no node owns it.
        pub node: Option<NodeId>,
        /// The decoded image whose pixels are drawn.
        pub source: crate::kernel::ImageSource,
        /// How the image is scaled to fill `destination`.
        pub fit: crate::kernel::ObjectFit,
        /// The rectangle the image is fitted into, in logical pixels.
        pub destination: Rect,
        /// The opacity in basis points; 10000 is opaque.
        pub opacity: u16,
        /// The band of image rows to draw; `None` draws the whole image.
        pub crop: Option<crate::kernel::ImageCrop>,
        /// A source that `source_at` asks for pixels at the exact device size, used only when `crop` is `None`.
        pub raster: Option<crate::kernel::ImageRaster>,
    }
}

#[cfg(raster)]
const RASTER_MAX_SIDE: f32 = 4096.0;

impl DrawImage {
    kittest_pub! {
        if feature = "scene";
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
}
