#![allow(clippy::redundant_pub_crate)]

use crate::scene::corner_radii::CornerRadii;
use crate::scene::shadow::Shadow;

mod clip;
#[cfg(scene_host)]
mod decoration;
pub(crate) mod effects;
pub(crate) mod fills;
mod glyph;
pub(crate) mod gradient;
pub(crate) mod image_fit;
pub(crate) mod payloads;
#[cfg(scene_host)]
pub(crate) mod stroke_band;
pub(crate) mod transform3d;
#[cfg(scene_host)]
pub(crate) mod trim;
mod wash;

kittest_pub! {
    #[doc(hidden)]
    pub use clip::Clip;
}
#[cfg(scene_host)]
pub(crate) use decoration::DecorationGeometry;
pub use effects::BlendMode;
kittest_pub! {
    #[doc(hidden)]
    pub use effects::{
        DecorationKind, Filter, LayerEffect, LayerScope, Mask, MaskMode, Transform,
    };
}
kittest_pub! {
    #[doc(hidden)]
    #[cfg(scene_paint)]
    pub use effects::{SpriteKind, SpriteSource};
}
#[cfg(feature = "kittest")]
pub use fills::PaintFill;
pub(crate) use fills::{FillRule, PathPaint};
pub use fills::{StrokeCap, StrokeJoin};
kittest_pub! {
    #[doc(hidden)]
    pub use fills::{GradientStop, LinearGradient, PatternKind, Stroke, StrokeAlign, StrokeStyle};
}
pub use glyph::GlyphRun;
kittest_pub! {
    #[doc(hidden)]
    pub use glyph::TextFlow;
}
#[cfg(scene_host)]
kittest_pub! {
    #[doc(hidden)]
    pub use glyph::shape_glyph_run;
}
pub(crate) use glyph::{RunInk, shape_run};
kittest_pub! {
    #[doc(hidden)]
    pub use gradient::{Gradient, GradientKind};
}
pub(crate) use payloads::CellRect;
#[cfg(vector_paint)]
pub(crate) use payloads::MITER_LIMIT;
kittest_pub! {
    #[doc(hidden)]
    pub use payloads::CellBorder;
}
pub use payloads::DrawRect;
kittest_pub! {
    #[doc(hidden)]
    pub use payloads::{DrawDecoration, DrawGradient, DrawImage, DrawPath, DrawPattern};
}
kittest_pub! {
    #[doc(hidden)]
    #[cfg(scene_paint)]
    pub use payloads::DrawSprite;
}
pub(crate) use payloads::{PathCmd, PathStroke, outline_key, rounded_outline};
#[cfg(scene_host)]
pub(crate) use stroke_band::StrokeBand;
#[cfg(scene_harness)]
pub(crate) use stroke_band::StrokeDash;
kittest_pub! {
    #[doc(hidden)]
    pub use transform3d::Transform3D;
}
pub use wash::Wash;
#[cfg(scene_host)]
pub(crate) use wash::{MAX_PIGMENTS, centre_bits as wash_centre_bits};

/// The ordered drawing commands of one frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommandScene {
    #[cfg(feature = "kittest")]
    #[doc(hidden)]
    pub commands: Vec<SceneCommand>,
    #[cfg(not(feature = "kittest"))]
    pub(crate) commands: Vec<SceneCommand>,
}

impl CommandScene {
    #[must_use]
    pub(crate) const fn new(commands: Vec<SceneCommand>) -> Self {
        Self { commands }
    }

    /// The text runs of this scene, in command order.
    pub fn glyph_runs(&self) -> impl Iterator<Item = &GlyphRun> {
        self.commands.iter().filter_map(|command| match command {
            SceneCommand::GlyphRun(draw) => Some(draw),
            _ => None,
        })
    }

    /// Returns the first text run whose text equals `expected`.
    #[must_use]
    pub fn find_text(&self, expected: &str) -> Option<&GlyphRun> {
        self.glyph_runs().find(|draw| draw.text() == expected)
    }
}

kittest_pub! {
    /// One draw or state command in a `CommandScene`.
    #[doc(hidden)]
    #[derive(Clone, Debug, PartialEq)]
    #[non_exhaustive]
    pub enum SceneCommand {
        /// Clips the commands up to the matching `PopClip`.
        PushClip(Clip),
        /// Ends the clip that the matching `PushClip` opened.
        PopClip,
        #[cfg(scene_paint)]
        /// Applies a 2D affine transform to the commands up to the matching `PopTransform`.
        PushTransform(Transform),
        #[cfg(scene_paint)]
        /// Ends the transform that the matching `PushTransform` opened.
        PopTransform,
        #[cfg(host)]
        /// Draws the commands up to the matching `RestoreLayer` into a layer composited at `opacity`.
        SaveLayer {
            /// The opacity the layer is composited at, from 0 to 1.
            opacity: f32,
        },
        #[cfg(terminal_host)]
        /// Ends the layer that the matching `SaveLayer` opened.
        RestoreLayer,
        /// Draws the commands up to the matching `PopLayer` into a layer with its own clip, effect, opacity, and blend mode.
        PushLayer(LayerScope),
        /// Ends the layer that the matching `PushLayer` opened.
        PopLayer,
        /// Fills or strokes a rounded rectangle with one color.
        Rect(DrawRect),
        /// Fills a rounded rectangle with a gradient.
        Gradient(DrawGradient),
        /// Fills a rounded rectangle with a repeating pattern.
        Pattern(DrawPattern),
        /// Draws the shadow of a rounded rectangle.
        DrawShadow {
            /// The rectangle that casts the shadow, as x, y, width, and height.
            rect: [f32; 4],
            /// The corner radii of the rectangle that casts the shadow.
            radii: CornerRadii,
            /// The offset, blur, spread, color, and kind of the shadow.
            shadow: Shadow,
        },
        /// Draws one run of text.
        GlyphRun(GlyphRun),
        #[cfg(feature = "kittest")]
        /// Draws a region of an atlas texture into a rectangle.
        Sprite(DrawSprite),
        /// Draws an image fitted into a rectangle.
        Image(DrawImage),
        /// Fills and strokes a vector path.
        Path(DrawPath),
        /// Draws an underline or a strikethrough.
        Decoration(DrawDecoration),
    }
}

impl SceneCommand {
    #[cfg(feature = "scene")]
    pub(crate) const fn is_image(&self) -> bool {
        match self {
            Self::Image(_) => true,
            #[cfg(feature = "kittest")]
            Self::Sprite(_) => true,
            _ => false,
        }
    }
}
