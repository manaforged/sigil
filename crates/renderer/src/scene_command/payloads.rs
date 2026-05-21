//! Draw payload structs and scene style types for each draw command.

#![allow(clippy::redundant_pub_crate)]

use hal::Rect;
use kernel::NodeId;
use style::Rgba8;

use crate::corner_radii::CornerRadii;

use super::effects::{DecorationKind, SpriteKind, SpriteSource};
use super::fills::{LinearGradient, PatternKind, Stroke};

/// Fill style for a rectangle draw command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneRectStyle {
    /// Resolved fill colour.
    pub color: Rgba8,
    /// Corner radii in top-left, top-right, bottom-right, bottom-left order.
    pub border_radius: CornerRadii,
    /// Opacity encoded as `value * 10000`.
    pub opacity: u16,
    /// Optional solid-colour stroke.
    pub stroke: Option<Stroke>,
}

impl SceneRectStyle {
    /// Create a rectangle fill style.
    #[must_use]
    pub const fn new(color: Rgba8, border_radius: CornerRadii, opacity: u16) -> Self {
        Self {
            color,
            border_radius,
            opacity,
            stroke: None,
        }
    }

    /// Fully-opaque solid fill with no corner radii.
    #[must_use]
    pub const fn solid(color: Rgba8) -> Self {
        Self::new(color, CornerRadii::ZERO, 10_000)
    }

    /// Fully-opaque fill with uniform corner radii.
    #[must_use]
    pub const fn rounded(color: Rgba8, radius: f32) -> Self {
        Self::new(color, CornerRadii::uniform(radius), 10_000)
    }

    /// Return this style with an added stroke.
    #[must_use]
    pub const fn with_stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }
}

/// Fill style for a linear-gradient rectangle draw command.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneGradientStyle {
    /// Resolved linear gradient.
    pub gradient: LinearGradient,
    /// Corner radii in top-left, top-right, bottom-right, bottom-left order.
    pub border_radius: CornerRadii,
    /// Opacity encoded as `value * 10000`.
    pub opacity: u16,
    /// Optional solid-colour stroke.
    pub stroke: Option<Stroke>,
}

impl SceneGradientStyle {
    /// Create a gradient fill style.
    #[must_use]
    pub const fn new(gradient: LinearGradient, border_radius: CornerRadii, opacity: u16) -> Self {
        Self {
            gradient,
            border_radius,
            opacity,
            stroke: None,
        }
    }

    /// Return this style with an added stroke.
    #[must_use]
    pub const fn with_stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }
}

/// Fill style for a pattern rectangle draw command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScenePatternStyle {
    /// Resolved pattern kind.
    pub pattern: PatternKind,
    /// Corner radii in top-left, top-right, bottom-right, bottom-left order.
    pub border_radius: CornerRadii,
    /// Opacity encoded as `value * 10000`.
    pub opacity: u16,
    /// Optional solid-colour stroke.
    pub stroke: Option<Stroke>,
}

impl ScenePatternStyle {
    /// Create a pattern fill style.
    #[must_use]
    pub const fn new(pattern: PatternKind, border_radius: CornerRadii, opacity: u16) -> Self {
        Self {
            pattern,
            border_radius,
            opacity,
            stroke: None,
        }
    }

    /// Return this style with an added stroke.
    #[must_use]
    pub const fn with_stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }
}

/// Style parameters for a sprite draw command — mirrors `DrawSprite` for adapters.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneSpriteStyle {
    /// Source atlas region.
    pub source: SpriteSource,
    /// Sprite rendering mode.
    pub kind: SpriteKind,
    /// Destination rectangle in scene logical pixels.
    pub destination: Rect,
    /// Tint color for `Mono` and `Subpixel`; ignored for `Poly`.
    pub color: Rgba8,
    /// Opacity encoded as `value * 10000`.
    pub opacity: u16,
}

/// Style parameters for a decoration draw command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneDecorationStyle {
    /// Which decoration to render.
    pub kind: DecorationKind,
    /// Horizontal band rect — width = full text run width, height = line thickness.
    pub band: Rect,
    /// Decoration colour in sRGB.
    pub color: Rgba8,
    /// Opacity encoded as `value * 10000`.
    pub opacity: u16,
    /// Logical px between wavy peaks; ignored for `Underline` and `Strikethrough`.
    pub period: f32,
    /// Logical px of stroke thickness within the band.
    pub line_width: f32,
}

/// Filled rectangle draw payload.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawRect {
    /// Source node, when emitted from a core tree.
    pub node: Option<NodeId>,
    /// Rectangle in scene logical pixels.
    pub rect: Rect,
    /// Resolved fill colour.
    pub color: Rgba8,
    /// Corner radii in top-left, top-right, bottom-right, bottom-left order.
    pub border_radius: CornerRadii,
    /// Opacity encoded as `value * 10000`.
    pub opacity: u16,
    /// Optional solid-colour stroke.
    pub stroke: Option<Stroke>,
}

impl DrawRect {
    /// Create a rectangle draw payload.
    #[must_use]
    pub const fn new(rect: Rect, style: SceneRectStyle) -> Self {
        Self {
            node: None,
            rect,
            color: style.color,
            border_radius: style.border_radius,
            opacity: style.opacity,
            stroke: style.stroke,
        }
    }

    pub(crate) const fn with_node(mut self, node: NodeId) -> Self {
        self.node = Some(node);
        self
    }
}

/// Linear-gradient rectangle draw payload.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawGradient {
    /// Source node, when emitted from a core tree.
    pub node: Option<NodeId>,
    /// Rectangle in scene logical pixels.
    pub rect: Rect,
    /// Resolved linear gradient.
    pub gradient: LinearGradient,
    /// Corner radii in top-left, top-right, bottom-right, bottom-left order.
    pub border_radius: CornerRadii,
    /// Opacity encoded as `value * 10000`.
    pub opacity: u16,
    /// Optional solid-colour stroke.
    pub stroke: Option<Stroke>,
}

impl DrawGradient {
    /// Create a gradient rectangle draw payload.
    #[must_use]
    pub fn new(rect: Rect, style: SceneGradientStyle) -> Self {
        Self {
            node: None,
            rect,
            gradient: style.gradient,
            border_radius: style.border_radius,
            opacity: style.opacity,
            stroke: style.stroke,
        }
    }
}

/// Pattern-filled rectangle draw payload.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawPattern {
    /// Source node, when emitted from a core tree.
    pub node: Option<NodeId>,
    /// Rectangle in scene logical pixels.
    pub rect: Rect,
    /// Resolved pattern kind.
    pub pattern: PatternKind,
    /// Corner radii in top-left, top-right, bottom-right, bottom-left order.
    pub border_radius: CornerRadii,
    /// Opacity encoded as `value * 10000`.
    pub opacity: u16,
    /// Optional solid-colour stroke.
    pub stroke: Option<Stroke>,
}

impl DrawPattern {
    /// Create a pattern rectangle draw payload.
    #[must_use]
    pub const fn new(rect: Rect, style: ScenePatternStyle) -> Self {
        Self {
            node: None,
            rect,
            pattern: style.pattern,
            border_radius: style.border_radius,
            opacity: style.opacity,
            stroke: style.stroke,
        }
    }

    #[allow(dead_code)]
    pub(crate) const fn with_node(mut self, node: NodeId) -> Self {
        self.node = Some(node);
        self
    }
}

/// Sprite draw payload emitted into a command scene.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawSprite {
    /// Source node, when emitted from a core tree.
    pub node: Option<NodeId>,
    /// Source atlas region.
    pub source: SpriteSource,
    /// Sprite rendering mode.
    pub kind: SpriteKind,
    /// Destination rectangle in scene logical pixels.
    pub destination: Rect,
    /// Tint color for `Mono` and `Subpixel`; ignored for `Poly`.
    pub color: Rgba8,
    /// Opacity encoded as `value * 10000`.
    pub opacity: u16,
}

impl DrawSprite {
    /// Create a sprite draw payload from a style descriptor.
    #[must_use]
    pub const fn new(style: &SceneSpriteStyle) -> Self {
        Self {
            node: None,
            source: style.source,
            kind: style.kind,
            destination: style.destination,
            color: style.color,
            opacity: style.opacity,
        }
    }
}

/// Decoration draw payload emitted into a command scene.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawDecoration {
    /// Source node, when emitted from a core tree.
    pub node: Option<NodeId>,
    /// Which decoration to render.
    pub kind: DecorationKind,
    /// Horizontal band rect.
    pub band: Rect,
    /// Decoration colour in sRGB.
    pub color: Rgba8,
    /// Opacity encoded as `value * 10000`.
    pub opacity: u16,
    /// Logical px between wavy peaks.
    pub period: f32,
    /// Logical px of stroke thickness within the band.
    pub line_width: f32,
}
