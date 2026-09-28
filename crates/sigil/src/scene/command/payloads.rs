#![allow(clippy::redundant_pub_crate)]

use crate::hal::Rect;
use crate::kernel::NodeId;
use crate::style::{Rgba8, Tone};

use crate::scene::corner_radii::CornerRadii;

use super::effects::{DecorationKind, SpriteKind, SpriteSource};
use super::fills::{LinearGradient, PatternKind, Stroke};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneRectStyle {
    pub color: Rgba8,
    pub border_radius: CornerRadii,
    pub opacity: u16,
    pub stroke: Option<Stroke>,
}

impl SceneRectStyle {
    #[must_use]
    pub const fn new(color: Rgba8, border_radius: CornerRadii, opacity: u16) -> Self {
        Self {
            color,
            border_radius,
            opacity,
            stroke: None,
        }
    }

    #[must_use]
    pub const fn solid(color: Rgba8) -> Self {
        Self::new(color, CornerRadii::ZERO, 10_000)
    }

    #[must_use]
    pub const fn rounded(color: Rgba8, radius: f32) -> Self {
        Self::new(color, CornerRadii::uniform(radius), 10_000)
    }

    #[must_use]
    pub const fn with_stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneGradientStyle {
    pub gradient: LinearGradient,
    pub border_radius: CornerRadii,
    pub opacity: u16,
    pub stroke: Option<Stroke>,
}

impl SceneGradientStyle {
    #[must_use]
    pub const fn new(gradient: LinearGradient, border_radius: CornerRadii, opacity: u16) -> Self {
        Self {
            gradient,
            border_radius,
            opacity,
            stroke: None,
        }
    }

    #[must_use]
    pub const fn with_stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScenePatternStyle {
    pub pattern: PatternKind,
    pub border_radius: CornerRadii,
    pub opacity: u16,
    pub stroke: Option<Stroke>,
}

impl ScenePatternStyle {
    #[must_use]
    pub const fn new(pattern: PatternKind, border_radius: CornerRadii, opacity: u16) -> Self {
        Self {
            pattern,
            border_radius,
            opacity,
            stroke: None,
        }
    }

    #[must_use]
    pub const fn with_stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneSpriteStyle {
    pub source: SpriteSource,
    pub kind: SpriteKind,
    pub destination: Rect,
    pub color: Rgba8,
    pub opacity: u16,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneDecorationStyle {
    pub kind: DecorationKind,
    pub band: Rect,
    pub color: Rgba8,
    pub opacity: u16,
    pub period: f32,
    pub line_width: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawRect {
    pub node: Option<NodeId>,
    pub rect: Rect,
    pub color: Rgba8,
    pub border_radius: CornerRadii,
    pub opacity: u16,
    pub stroke: Option<Stroke>,
    pub tone: Option<Tone>,
    pub leaf: bool,
    pub border: Option<CellBorder>,
    pub outline: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellBorder {
    pub rect: Rect,
    pub widths: [f32; 4],
    pub color: Rgba8,
    pub rounded: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CellRect {
    pub tone: Option<Tone>,
    pub leaf: bool,
    pub border: Option<CellBorder>,
    pub outline: bool,
}

impl DrawRect {
    #[must_use]
    pub const fn new(rect: Rect, style: SceneRectStyle) -> Self {
        Self {
            node: None,
            rect,
            color: style.color,
            border_radius: style.border_radius,
            opacity: style.opacity,
            stroke: style.stroke,
            tone: None,
            leaf: false,
            border: None,
            outline: false,
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

#[derive(Clone, Debug, PartialEq)]
pub struct DrawGradient {
    pub node: Option<NodeId>,
    pub rect: Rect,
    pub gradient: LinearGradient,
    pub border_radius: CornerRadii,
    pub opacity: u16,
    pub stroke: Option<Stroke>,
    pub tone: Option<Tone>,
    pub leaf: bool,
    pub border: Option<CellBorder>,
    pub outline: bool,
}

impl DrawGradient {
    #[must_use]
    pub fn new(rect: Rect, style: SceneGradientStyle) -> Self {
        Self {
            node: None,
            rect,
            gradient: style.gradient,
            border_radius: style.border_radius,
            opacity: style.opacity,
            stroke: style.stroke,
            tone: None,
            leaf: false,
            border: None,
            outline: false,
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawPattern {
    pub node: Option<NodeId>,
    pub rect: Rect,
    pub pattern: PatternKind,
    pub border_radius: CornerRadii,
    pub opacity: u16,
    pub stroke: Option<Stroke>,
}

impl DrawPattern {
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
}

#[derive(Clone, Debug, PartialEq)]
pub struct DrawImage {
    pub node: Option<NodeId>,
    pub source: crate::kernel::ImageSource,
    pub fit: crate::kernel::ObjectFit,
    pub destination: Rect,
    pub opacity: u16,
    pub crop: Option<crate::kernel::ImageCrop>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DrawSprite {
    pub node: Option<NodeId>,
    pub source: SpriteSource,
    pub kind: SpriteKind,
    pub destination: Rect,
    pub color: Rgba8,
    pub opacity: u16,
}

impl DrawSprite {
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawDecoration {
    pub node: Option<NodeId>,
    pub kind: DecorationKind,
    pub band: Rect,
    pub color: Rgba8,
    pub opacity: u16,
    pub period: f32,
    pub line_width: f32,
}
