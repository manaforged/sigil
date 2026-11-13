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

kittest_pub! {
    /// A text decoration.
    #[doc(hidden)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct DrawDecoration {
        /// The node this decoration paints, or `None` when no node owns it.
        pub node: Option<NodeId>,
        /// Whether the line is an underline or a strikethrough.
        pub kind: DecorationKind,
        /// The rectangle the line is drawn in, centred on its height.
        pub band: Rect,
        /// The color that the line is painted with.
        pub color: Color,
        /// The opacity in basis points; 10000 is opaque.
        pub opacity: u16,
        /// The horizontal length of one wave of a wavy underline; `new` sets 0, which a straight line ignores.
        pub period: f32,
        /// The thickness of the line, at most the height of `band`; `new` sets 1.
        pub line_width: f32,
    }
}

impl DrawDecoration {
    kittest_pub! {
        /// A straight text decoration one pixel wide at full opacity.
        #[must_use]
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

    kittest_pub! {
        /// Sets the opacity in basis points; 10000 is opaque.
        #[must_use]
        pub const fn opacity(mut self, opacity: u16) -> Self {
            self.opacity = opacity;
            self
        }
    }

    kittest_pub! {
        /// Sets the width of the line; the default is 1.
        #[must_use]
        pub const fn line_width(mut self, line_width: f32) -> Self {
            self.line_width = line_width;
            self
        }
    }
}
