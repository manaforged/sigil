use crate::geometry::Rect;
use crate::kernel::NodeId;
use crate::style::Rgba8;

use crate::scene::command::effects::{DecorationKind, SpriteKind, SpriteSource};

/// A command that draws a sprite.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawSprite {
    /// The node, if any.
    pub node: Option<NodeId>,
    /// The source.
    pub source: SpriteSource,
    /// The kind.
    pub kind: SpriteKind,
    /// The destination.
    pub destination: Rect,
    /// The color.
    pub color: Rgba8,
    /// The opacity.
    pub opacity: u16,
}

impl DrawSprite {
    /// An atlas region drawn into `destination` at full opacity.
    #[must_use]
    pub const fn new(
        source: SpriteSource,
        destination: Rect,
        kind: SpriteKind,
        color: Rgba8,
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

    /// Sets the opacity in basis points; 10000 is opaque.
    #[must_use]
    pub const fn opacity(mut self, opacity: u16) -> Self {
        self.opacity = opacity;
        self
    }
}

/// A text decoration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawDecoration {
    /// The node, if any.
    pub node: Option<NodeId>,
    /// The kind.
    pub kind: DecorationKind,
    /// The band.
    pub band: Rect,
    /// The color.
    pub color: Rgba8,
    /// The opacity.
    pub opacity: u16,
    /// The period.
    pub period: f32,
    /// The line width.
    pub line_width: f32,
}

impl DrawDecoration {
    /// A straight text decoration one pixel wide at full opacity.
    #[must_use]
    pub const fn new(kind: DecorationKind, band: Rect, color: Rgba8) -> Self {
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

    /// Sets the opacity in basis points; 10000 is opaque.
    #[must_use]
    pub const fn opacity(mut self, opacity: u16) -> Self {
        self.opacity = opacity;
        self
    }

    /// Sets the wave period of a wavy decoration.
    #[must_use]
    pub const fn period(mut self, period: f32) -> Self {
        self.period = period;
        self
    }

    /// Sets the line width.
    #[must_use]
    pub const fn line_width(mut self, line_width: f32) -> Self {
        self.line_width = line_width;
        self
    }
}
