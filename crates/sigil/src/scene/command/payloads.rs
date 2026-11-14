#![allow(clippy::redundant_pub_crate)]

mod marks;
mod path;

pub use marks::DrawDecoration;
kittest_pub! {
    #[doc(hidden)]
    #[cfg(scene_paint)]
    pub use marks::DrawSprite;
}
#[cfg(vector_paint)]
pub(crate) use path::MITER_LIMIT;
pub use path::{DrawPath, PathCommand, PathStroke};
pub(crate) use path::{hash_path_cmd, outline_key, rounded_outline};

use crate::geometry::{Point, Rect};
use crate::kernel::NodeId;
use crate::node::{ImageSource, ObjectFit};
use crate::style::{Color, Tone};

use crate::scene::corner_radii::CornerRadii;

use super::Wash;
use super::fills::{PatternKind, Stroke};
use super::gradient::{Gradient, GradientKind};

/// A command that fills or strokes a rounded rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use]
pub struct DrawRect {
    /// The node this rectangle paints, or `None` when no node owns it.
    pub(crate) node: Option<NodeId>,
    /// The bounds of the rectangle, in logical pixels.
    pub(crate) rect: Rect,
    /// The fill color; a transparent color leaves only the stroke.
    pub(crate) color: Color,
    /// The radius of each corner.
    pub(crate) border_radius: CornerRadii,
    /// The opacity in basis points; 10000 is opaque.
    pub(crate) opacity: u16,
    /// The stroke around the rectangle, if any.
    pub(crate) stroke: Option<Stroke>,
    /// The theme tone the terminal host fills with when `color` is fully transparent.
    pub(crate) tone: Option<Tone>,
    /// True when the node has no laid-out child, so the terminal host may fill a partly covered cell with a half block.
    pub(crate) leaf: bool,
    /// The box-drawing border the terminal host draws around the node, if it has one.
    pub(crate) border: Option<CellBorder>,
    /// True for a focus ring or a border strip, which the terminal host does not fill.
    pub(crate) outline: bool,
}

/// A box-drawing border on terminal cells.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellBorder {
    pub(crate) rect: Rect,
    pub(crate) widths: [f32; 4],
    pub(crate) color: Color,
    pub(crate) rounded: bool,
}

impl CellBorder {
    /// Returns the outer bounds of the bordered box, in logical pixels.
    #[must_use]
    pub const fn rect(&self) -> Rect {
        self.rect
    }

    /// Returns the widths of the top, right, bottom, and left edges; an edge of 0 or less is not drawn.
    #[must_use]
    pub const fn widths(&self) -> [f32; 4] {
        self.widths
    }

    /// Returns the color of the box-drawing characters; a fully transparent color draws no border.
    #[must_use]
    pub const fn color(&self) -> Color {
        self.color
    }

    /// Returns true when the box has a corner radius, so the terminal host draws rounded corner characters.
    #[must_use]
    pub const fn is_rounded(&self) -> bool {
        self.rounded
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

            /// Returns the theme tone the terminal host fills with when the color is fully transparent.
            #[must_use]
            pub const fn tone(&self) -> Option<Tone> {
                self.tone
            }

            /// Returns true when the owning node has no laid-out child.
            #[must_use]
            pub const fn is_leaf(&self) -> bool {
                self.leaf
            }

            /// Returns true when this command is a focus ring or border strip, which the terminal host does not fill.
            #[must_use]
            pub const fn is_outline(&self) -> bool {
                self.outline
            }

            /// Returns the box-drawing border the terminal host draws around the node, if it has one.
            #[must_use]
            pub const fn border(&self) -> Option<CellBorder> {
                self.border
            }
        }
    )+};
}

node_and_cell!(DrawRect, DrawGradient);

macro_rules! shaped_box {
    ($($command:ty),+) => {$(
        impl $command {
            /// Sets the radius of each corner `[top_left, top_right, bottom_right, bottom_left]`.
            pub const fn radii(mut self, radii: [f32; 4]) -> Self {
                self.border_radius = CornerRadii(radii);
                self
            }

            /// Sets a uniform radius for all four corners.
            pub const fn radius(mut self, radius: f32) -> Self {
                self.border_radius = CornerRadii::uniform(radius);
                self
            }

            /// Sets the opacity in basis points; 10000 is opaque.
            pub const fn opacity(mut self, opacity: u16) -> Self {
                self.opacity = opacity;
                self
            }

            /// Returns the owning node, if any.
            #[must_use]
            pub const fn node(&self) -> Option<NodeId> {
                self.node
            }

            /// Returns the rectangle bounds in logical pixels.
            #[must_use]
            pub const fn rect(&self) -> Rect {
                self.rect
            }

            /// Returns the four corner radii `[top_left, top_right, bottom_right, bottom_left]`.
            #[must_use]
            pub const fn corner_radii(&self) -> [f32; 4] {
                self.border_radius.0
            }

            /// Returns the opacity in basis points; 10000 is opaque.
            #[must_use]
            pub const fn opacity_bp(&self) -> u16 {
                self.opacity
            }

            #[cfg(feature = "kittest")]
            /// Returns the corner radii wrapper.
            #[doc(hidden)]
            #[must_use]
            pub const fn border_radius_raw(&self) -> CornerRadii {
                self.border_radius
            }

            /// Returns the line drawn along the edge of the rectangle, if it has one.
            #[must_use]
            pub const fn stroke(&self) -> Option<Stroke> {
                self.stroke
            }
        }
    )+};
}

shaped_box!(DrawRect, DrawGradient, DrawPattern);

impl DrawRect {
    /// Creates a solid rectangle with square corners at full opacity.
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

    /// Returns the fill color.
    #[must_use]
    pub const fn color(&self) -> Color {
        self.color
    }

    kittest_pub! {
        /// Sets the radius of each corner of the rectangle.
        #[doc(hidden)]
        pub const fn border_radius(mut self, border_radius: CornerRadii) -> Self {
            self.border_radius = border_radius;
            self
        }
    }

    kittest_pub! {
        /// Adds a stroke to the rectangle.
        #[doc(hidden)]
        pub const fn stroked(mut self, stroke: Stroke) -> Self {
            self.stroke = Some(stroke);
            self
        }
    }
}

/// A command that fills a shape with a gradient.
#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct DrawGradient {
    /// The node this gradient paints, or `None` when no node owns it.
    pub(crate) node: Option<NodeId>,
    /// The bounds of the filled rectangle, in logical pixels.
    pub(crate) rect: Rect,
    /// The geometry and color stops of the fill.
    pub(crate) gradient: Gradient,
    /// The radius of each corner of the filled rectangle.
    pub(crate) border_radius: CornerRadii,
    /// The opacity in basis points; 10000 is opaque.
    pub(crate) opacity: u16,
    /// The line drawn along the edge of the rectangle, if it has one.
    pub(crate) stroke: Option<Stroke>,
    /// The theme tone the terminal host fills with when the gradient flattens to a fully transparent color.
    pub(crate) tone: Option<Tone>,
    /// True when the node has no laid-out child, so the terminal host may fill a partly covered cell with a half block.
    pub(crate) leaf: bool,
    /// The box-drawing border the terminal host draws around the node, if it has one.
    pub(crate) border: Option<CellBorder>,
    /// True for a focus ring or a border strip, which the terminal host does not fill.
    pub(crate) outline: bool,
}

impl DrawGradient {
    /// Creates a linear gradient fill across `rect` from `start` to `end`.
    pub fn linear(rect: Rect, start: Point, end: Point, stops: &[(f32, Color)]) -> Self {
        Self::new(
            rect,
            Gradient::from_tuples(GradientKind::Linear { start, end }, stops),
        )
    }

    /// Creates a radial gradient fill across `rect` centered at `center` with `radius`.
    pub fn radial(rect: Rect, center: Point, radius: f32, stops: &[(f32, Color)]) -> Self {
        Self::new(
            rect,
            Gradient::from_tuples(
                GradientKind::Radial {
                    center,
                    radius: [radius, radius],
                },
                stops,
            ),
        )
    }

    kittest_pub! {
        /// A gradient rectangle with square corners at full opacity.
        #[doc(hidden)]
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

    /// Returns the geometry and color stops of the fill.
    #[must_use]
    pub const fn gradient(&self) -> &Gradient {
        &self.gradient
    }

    kittest_pub! {
        /// Sets the radius of each corner of the gradient fill.
        #[doc(hidden)]
        pub const fn border_radius(mut self, border_radius: CornerRadii) -> Self {
            self.border_radius = border_radius;
            self
        }
    }

    kittest_pub! {
        /// Adds a stroke to the gradient fill.
        #[doc(hidden)]
        pub const fn stroked(mut self, stroke: Stroke) -> Self {
            self.stroke = Some(stroke);
            self
        }
    }
}

/// A command that fills a shape with a pattern.
#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use]
pub struct DrawPattern {
    /// The node this pattern paints, or `None` when no node owns it.
    pub(crate) node: Option<NodeId>,
    /// The bounds of the filled rectangle, in logical pixels; the pattern repeats from its origin.
    pub(crate) rect: Rect,
    /// The shape and colors of the repeating fill.
    pub(crate) pattern: PatternKind,
    /// The radius of each corner of the filled rectangle.
    pub(crate) border_radius: CornerRadii,
    /// The opacity in basis points; 10000 is opaque.
    pub(crate) opacity: u16,
    /// The line drawn along the edge of the rectangle, if it has one.
    pub(crate) stroke: Option<Stroke>,
}

impl DrawPattern {
    /// Creates a watercolour wash fill over `rect`.
    pub const fn wash(rect: Rect, wash: Wash) -> Self {
        Self::new(rect, PatternKind::Wash(wash))
    }

    kittest_pub! {
        /// A pattern rectangle with square corners at full opacity.
        #[doc(hidden)]
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

    /// Returns the shape and colors of the repeating fill.
    #[must_use]
    pub const fn pattern(&self) -> PatternKind {
        self.pattern
    }

    #[cfg(feature = "kittest")]
    /// Sets the radius of each corner of the pattern fill.
    #[doc(hidden)]
    pub const fn border_radius(mut self, border_radius: CornerRadii) -> Self {
        self.border_radius = border_radius;
        self
    }

    #[cfg(feature = "kittest")]
    /// Adds a stroke to the pattern fill.
    #[doc(hidden)]
    pub const fn stroked(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }
}

/// An image command fitted into a destination rectangle, with optional crop.
#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct DrawImage {
    /// The node this image paints, or `None` when no node owns it.
    pub(crate) node: Option<NodeId>,
    /// The decoded image whose pixels are drawn.
    pub(crate) source: ImageSource,
    /// How the image is scaled to fill `destination`.
    pub(crate) fit: ObjectFit,
    /// The rectangle the image is fitted into, in logical pixels.
    pub(crate) destination: Rect,
    /// The opacity in basis points; 10000 is opaque.
    pub(crate) opacity: u16,
    /// The band of image rows to draw; `None` draws the whole image.
    pub(crate) crop: Option<crate::kernel::ImageCrop>,
    /// A source that `source_at` asks for pixels at the exact device size, used only when `crop` is `None`.
    pub(crate) raster: Option<crate::kernel::ImageRaster>,
}

#[cfg(raster)]
const RASTER_MAX_SIDE: f32 = 4096.0;

impl DrawImage {
    /// Creates an image command fitted to `destination` with `ObjectFit::Fill` at full opacity.
    pub const fn new(source: ImageSource, destination: Rect) -> Self {
        Self {
            node: None,
            source,
            fit: ObjectFit::Fill,
            destination,
            opacity: 10_000,
            crop: None,
            raster: None,
        }
    }

    /// Sets how the image scales to fit its destination rectangle.
    pub const fn fit(mut self, fit: ObjectFit) -> Self {
        self.fit = fit;
        self
    }

    /// Sets the opacity in basis points; 10000 is opaque.
    pub const fn opacity(mut self, opacity: u16) -> Self {
        self.opacity = opacity;
        self
    }

    /// Returns the owning node, if any.
    #[must_use]
    pub const fn node(&self) -> Option<NodeId> {
        self.node
    }

    /// Returns the decoded image source.
    #[must_use]
    pub const fn source(&self) -> &ImageSource {
        &self.source
    }

    /// Returns how the image scales to fit its destination rectangle.
    #[must_use]
    pub const fn object_fit(&self) -> ObjectFit {
        self.fit
    }

    /// Returns the destination rectangle in logical pixels.
    #[must_use]
    pub const fn destination(&self) -> Rect {
        self.destination
    }

    /// Returns the opacity in basis points; 10000 is opaque.
    #[must_use]
    pub const fn opacity_bp(&self) -> u16 {
        self.opacity
    }

    #[cfg(feature = "kittest")]
    /// Sets the source image crop.
    #[doc(hidden)]
    pub const fn crop(mut self, crop: Option<crate::kernel::ImageCrop>) -> Self {
        self.crop = crop;
        self
    }

    #[cfg(feature = "kittest")]
    /// Sets the size-aware raster source.
    #[doc(hidden)]
    pub fn raster(mut self, raster: Option<crate::kernel::ImageRaster>) -> Self {
        self.raster = raster;
        self
    }

    #[cfg(feature = "kittest")]
    /// Returns the source image crop, if any.
    #[doc(hidden)]
    #[must_use]
    pub const fn image_crop(&self) -> Option<crate::kernel::ImageCrop> {
        self.crop
    }

    #[must_use]
    pub(crate) fn source_at(&self, scale: f32) -> (ImageSource, Rect) {
        #[cfg(raster)]
        if let Some(sized) = self.raster_at(scale) {
            return sized;
        }
        #[cfg(not(raster))]
        let _ = scale;
        (self.source.clone(), self.destination)
    }

    #[cfg(raster)]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "the extent is clamped to a bounded positive range before the cast"
    )]
    fn raster_at(&self, scale: f32) -> Option<(ImageSource, Rect)> {
        let dest = self.destination;
        let raster = self
            .raster
            .as_ref()
            .filter(|_| self.crop.is_none() && scale.is_finite() && scale > 0.0)?;
        let left = (dest.origin.x * scale).round();
        let top = (dest.origin.y * scale).round();
        let width = ((dest.origin.x + dest.width) * scale).round() - left;
        let height = ((dest.origin.y + dest.height) * scale).round() - top;
        let side = |extent: f32| extent.clamp(1.0, RASTER_MAX_SIDE) as u32;
        let source = raster.at(side(width), side(height), scale)?;
        let snapped = Rect::new(
            Point::new(left / scale, top / scale),
            source.width() as f32 / scale,
            source.height() as f32 / scale,
        );
        Some((source, snapped))
    }
}
