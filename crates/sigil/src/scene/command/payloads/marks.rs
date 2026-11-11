use crate::geometry::Rect;
use crate::kernel::NodeId;
use crate::style::Color;

use crate::scene::command::effects::DecorationKind;
#[cfg(any(
    test,
    feature = "kittest",
    all(feature = "scene", any(feature = "desktop", feature = "terminal"))
))]
use crate::scene::command::effects::{SpriteKind, SpriteSource};

kittest_pub! {
    /// A command that draws a sprite.
    #[doc(hidden)]
    #[cfg(any(
        test,
        feature = "kittest",
        all(feature = "scene", any(feature = "desktop", feature = "terminal"))
    ))]
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
        pub color: Color,
        /// The opacity.
        pub opacity: u16,
    }
}

#[cfg(any(
    test,
    feature = "kittest",
    all(feature = "scene", any(feature = "desktop", feature = "terminal"))
))]
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
        /// The node, if any.
        pub node: Option<NodeId>,
        /// The kind.
        pub kind: DecorationKind,
        /// The band.
        pub band: Rect,
        /// The color.
        pub color: Color,
        /// The opacity.
        pub opacity: u16,
        /// The period.
        pub period: f32,
        /// The line width.
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
        /// Sets the line width.
        #[must_use]
        pub const fn line_width(mut self, line_width: f32) -> Self {
            self.line_width = line_width;
            self
        }
    }
}
