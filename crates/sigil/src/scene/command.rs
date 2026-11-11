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

    /// Returns the text runs of this scene in paint order.
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
        /// The pop clip scene command.
        PopClip,
        #[cfg(scene_paint)]
        /// The push transform scene command.
        PushTransform(Transform),
        #[cfg(scene_paint)]
        /// The pop transform scene command.
        PopTransform,
        #[cfg(host)]
        /// The save layer scene command.
        SaveLayer {
            /// The opacity.
            opacity: f32,
        },
        #[cfg(terminal_host)]
        /// The restore layer scene command.
        RestoreLayer,
        /// The push layer scene command.
        PushLayer(LayerScope),
        /// The pop layer scene command.
        PopLayer,
        /// The rect scene command.
        Rect(DrawRect),
        /// The gradient scene command.
        Gradient(DrawGradient),
        /// The pattern scene command.
        Pattern(DrawPattern),
        /// The draw shadow scene command.
        DrawShadow {
            /// The rect.
            rect: [f32; 4],
            /// The radii.
            radii: CornerRadii,
            /// The shadow.
            shadow: Shadow,
        },
        /// The glyph run scene command.
        GlyphRun(GlyphRun),
        #[cfg(feature = "kittest")]
        /// The sprite scene command.
        Sprite(DrawSprite),
        /// The image scene command.
        Image(DrawImage),
        /// The vector path scene command.
        Path(DrawPath),
        /// The decoration scene command.
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
