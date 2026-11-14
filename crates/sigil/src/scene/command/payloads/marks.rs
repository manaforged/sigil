use crate::geometry::Rect;
use crate::kernel::NodeId;
use crate::style::Color;

use crate::scene::command::effects::DecorationKind;
#[cfg(scene_paint)]
use crate::scene::command::effects::{SpriteKind, SpriteSource};

kittest_pub! {
    /// A command that draws a sprite.
    #[doc(hidden)]
    #[cfg(scene_paint)]
    #[derive(Clone, Debug, PartialEq)]
    pub struct DrawSprite {
        /// The node this sprite paints, or `None` when no node owns it.
        pub node: Option<NodeId>,
        /// The atlas and the region of it to draw.
        pub source: SpriteSource,
        /// Whether the atlas region is coverage to tint or full color.
        pub kind: SpriteKind,
        /// The rectangle the region is stretched into, in logical pixels.
        pub destination: Rect,
        /// The tint of a `Mono` or `Subpixel` sprite; a `Poly` sprite ignores it.
        pub color: Color,
        /// The opacity in basis points; 10000 is opaque.
        pub opacity: u16,
    }
}

#[cfg(scene_paint)]
impl DrawSprite {
    kittest_pub! {
        if test;
        /// An atlas region drawn into `destination` at full opacity.
        #[must_use]
        pub const fn new(
            source: SpriteSource,
            destination: Rect,
            kind: SpriteKind,
            color: Color,
        ) -> Self {
            Self {
                node: None,
                source,
                kind,
                destination,
                color,
                opacity: 10_000,
            }
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
}

/// A text decoration.
#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use]
pub struct DrawDecoration {
    /// The node this decoration paints, or `None` when no node owns it.
    pub(crate) node: Option<NodeId>,
    /// Whether the line is an underline or a strikethrough.
    pub(crate) kind: DecorationKind,
    /// The rectangle the line is drawn in, centred on its height.
    pub(crate) band: Rect,
    /// The color that the line is painted with.
    pub(crate) color: Color,
    /// The opacity in basis points; 10000 is opaque.
    pub(crate) opacity: u16,
    /// The horizontal length of one wave of a wavy underline; `new` sets 0, which a straight line ignores.
    pub(crate) period: f32,
    /// The thickness of the line, at most the height of `band`; `new` sets 1.
    pub(crate) line_width: f32,
}

impl DrawDecoration {
    /// Creates an underline decoration one pixel wide at full opacity.
    pub const fn underline(band: Rect, color: Color) -> Self {
        Self::new(DecorationKind::Underline, band, color)
    }

    /// Creates a strikethrough decoration one pixel wide at full opacity.
    pub const fn strikethrough(band: Rect, color: Color) -> Self {
        Self::new(DecorationKind::Strikethrough, band, color)
    }

    kittest_pub! {
        /// A straight text decoration one pixel wide at full opacity.
        #[doc(hidden)]
        pub const fn new(kind: DecorationKind, band: Rect, color: Color) -> Self {
            Self {
                node: None,
                kind,
                band,
                color,
                opacity: 10_000,
                period: 0.0,
                line_width: 1.0,
            }
        }
    }

    /// Sets the opacity in basis points; 10000 is opaque.
    pub const fn opacity(mut self, opacity: u16) -> Self {
        self.opacity = opacity;
        self
    }

    /// Sets the width of the line; the default is 1.
    pub const fn line_width(mut self, line_width: f32) -> Self {
        self.line_width = line_width;
        self
    }

    /// Sets the horizontal length of one wave of a wavy underline.
    pub const fn period(mut self, period: f32) -> Self {
        self.period = period;
        self
    }

    /// Returns the owning node, if any.
    #[must_use]
    pub const fn node(&self) -> Option<NodeId> {
        self.node
    }

    /// Returns the decoration band rectangle.
    #[must_use]
    pub const fn band(&self) -> Rect {
        self.band
    }

    /// Returns the decoration color.
    #[must_use]
    pub const fn color(&self) -> Color {
        self.color
    }

    /// Returns the opacity in basis points; 10000 is opaque.
    #[must_use]
    pub const fn opacity_bp(&self) -> u16 {
        self.opacity
    }

    /// Returns the stroke width of the decoration line.
    #[must_use]
    pub const fn width(&self) -> f32 {
        self.line_width
    }

    /// Returns the horizontal length of one wave of a wavy underline.
    #[must_use]
    pub const fn wave_period(&self) -> f32 {
        self.period
    }

    /// Returns whether the line is an underline or a strikethrough.
    #[must_use]
    pub const fn kind(&self) -> DecorationKind {
        self.kind
    }
}
