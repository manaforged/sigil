use std::hash::{Hash, Hasher};
use std::mem::Discriminant;

use super::{Color, ColorValue, GradientAxis, bits, direction};
use crate::style::diagnostic::Seen;
use crate::style::{AnimatorOverlay, ColorAtomKind, NumericAtomKind};

/// How far a radial gradient reaches, as in CSS `radial-gradient(<extent>)`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum RadialExtent {
    /// To the nearest side of the box.
    ClosestSide,
    /// Through the nearest corner of the box.
    ClosestCorner,
    /// To the farthest side of the box.
    FarthestSide,
    /// Through the farthest corner of the box; the CSS default.
    #[default]
    FarthestCorner,
}

/// The shape, size, and center of a radial gradient, like CSS `radial-gradient` and Tailwind `bg-radial`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Radial {
    pub(crate) circle: bool,
    pub(crate) extent: RadialExtent,
    pub(crate) at: [u8; 2],
    pub(crate) repeating: bool,
}

impl Radial {
    /// An ellipse through the farthest corner, centered in the box.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            circle: false,
            extent: RadialExtent::FarthestCorner,
            at: [50, 50],
            repeating: false,
        }
    }

    /// Sets whether the ending shape is a circle instead of an ellipse.
    #[must_use]
    pub const fn circle(mut self, circle: bool) -> Self {
        self.circle = circle;
        self
    }

    /// Sets how far the gradient reaches.
    #[must_use]
    pub const fn extent(mut self, extent: RadialExtent) -> Self {
        self.extent = extent;
        self
    }

    /// Sets the center in percent of the box width and height, each clamped to 0..=100.
    #[must_use]
    pub const fn at(mut self, x: u8, y: u8) -> Self {
        self.at = [percent(x), percent(y)];
        self
    }

    /// Sets whether the stops repeat, like CSS `repeating-radial-gradient`.
    #[must_use]
    pub const fn repeating(mut self, repeating: bool) -> Self {
        self.repeating = repeating;
        self
    }
}

impl Default for Radial {
    fn default() -> Self {
        Self::new()
    }
}

/// The start angle and center of a conic gradient, like CSS `conic-gradient` and Tailwind `bg-conic-*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Conic {
    pub(crate) from: i16,
    pub(crate) at: [u8; 2],
    pub(crate) repeating: bool,
}

impl Conic {
    /// A conic gradient centered in the box that starts `from_degrees` clockwise from up.
    #[must_use]
    pub const fn new(from_degrees: i16) -> Self {
        Self {
            from: from_degrees,
            at: [50, 50],
            repeating: false,
        }
    }

    /// Sets the center in percent of the box width and height, each clamped to 0..=100.
    #[must_use]
    pub const fn at(mut self, x: u8, y: u8) -> Self {
        self.at = [percent(x), percent(y)];
        self
    }

    /// Sets whether the stops repeat, like CSS `repeating-conic-gradient`.
    #[must_use]
    pub const fn repeating(mut self, repeating: bool) -> Self {
        self.repeating = repeating;
        self
    }
}

impl From<i16> for Conic {
    fn from(from_degrees: i16) -> Self {
        Self::new(from_degrees)
    }
}

pub(super) const fn percent(value: u8) -> u8 {
    if value > 100 { 100 } else { value }
}

kittest_pub! {
    #[doc(hidden)]
    /// The geometry of a background gradient.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum GradientShape {
        /// A linear gradient along a box axis.
        Linear(GradientAxis),
        /// A radial gradient.
        Radial(Radial),
        /// A conic gradient.
        Conic(Conic),
    }
}

kittest_pub! {
    #[doc(hidden)]
    /// One of the three Tailwind gradient stops.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum StopSlot {
        /// The `from-*` stop.
        From,
        /// The `via-*` stop.
        Via,
        /// The `to-*` stop.
        To,
    }
}

kittest_pub! {
    #[doc(hidden)]
    /// Selects how the line of a border is drawn: solid, dashed, or dotted.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub enum BorderStyle {
        /// A solid line.
        #[default]
        Solid,
        /// Dashes along the edge.
        Dashed,
        /// Round dots along the edge.
        Dotted,
    }
}

kittest_pub! {
    #[doc(hidden)]
    /// A paint style atom: gradients, stops, clip, image, and border style.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum PaintAtom {
        /// The shape of the background gradient: linear, radial, or conic.
        Gradient(GradientShape),
        /// No background gradient.
        NoGradient,
        /// A gradient stop color.
        Stop(StopSlot, ColorValue),
        /// No color at a gradient stop.
        NoStop(StopSlot),
        /// A gradient stop position in percent.
        StopAt(StopSlot, u8),
        /// No background color and no stop-list gradient.
        NoColor,
        /// Paints the background only inside the text glyphs.
        ClipText,
        /// A tiled background image, by image id.
        Image(u64),
        /// A background shader, by shader id.
        Shader(u64),
        /// How the line of the border is drawn: solid, dashed, or dotted.
        Border(BorderStyle),
    }
}

kittest_pub! {
    /// The resolved paint atoms of one node.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct PaintStyle {
        /// The shape of the background gradient, when one is set.
        pub gradient: Option<GradientShape>,
        /// The from, via, and to colors.
        pub stops: [Option<Color>; 3],
        /// The from, via, and to positions in percent.
        pub positions: [Option<u8>; 3],
        /// Whether the background shows only inside text.
        pub clip_text: bool,
        /// The id of the tiled background image, when one is set.
        pub image: Option<u64>,
        /// The id of the background shader, when one is set.
        pub shader: Option<u64>,
        /// How the line of the border is drawn; the default is solid.
        pub border: BorderStyle,
        pub(crate) geometry: Geometry,
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Geometry {
    angle: Option<f32>,
    center: [Option<f32>; 2],
}

impl Geometry {
    pub(crate) fn angle(self, degrees: f32) -> f32 {
        self.angle.unwrap_or(degrees)
    }

    pub(crate) fn direction(self, axis: GradientAxis) -> [f32; 2] {
        self.angle.map_or_else(|| axis.heading().1, direction)
    }

    pub(crate) fn center(self, at: [u8; 2]) -> [f32; 2] {
        let [x, y] = self.center;
        [x.unwrap_or(f32::from(at[0])), y.unwrap_or(f32::from(at[1]))]
    }

    fn overlay(&mut self, shape: GradientShape, overlay: &dyn AnimatorOverlay) {
        let Self {
            angle,
            center: [x, y],
        } = self;
        for (kind, slot) in [
            (NumericAtomKind::GradientAngle, angle),
            (NumericAtomKind::GradientCenterX, x),
            (NumericAtomKind::GradientCenterY, y),
        ] {
            let Some(value) = overlay.value_for(kind) else {
                continue;
            };
            if shape.value(kind).is_some() {
                *slot = Some(value);
            } else if REFUSED.first((kind, std::mem::discriminant(&shape))) {
                tracing::warn!(
                    target: "sigil::animation",
                    ?kind,
                    ?shape,
                    "this property has no meaning on this gradient and was ignored"
                );
            }
        }
    }

    const fn key(self) -> [u32; 3] {
        let [x, y] = self.center;
        [slot_key(self.angle), slot_key(x), slot_key(y)]
    }
}

const fn slot_key(value: Option<f32>) -> u32 {
    match value {
        Some(value) => bits(value),
        None => u32::MAX,
    }
}

impl PartialEq for Geometry {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

impl Eq for Geometry {}

impl Hash for Geometry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key().hash(state);
    }
}

static REFUSED: Seen<(NumericAtomKind, Discriminant<GradientShape>)> = Seen::new();

impl GradientShape {
    fn value(self, kind: NumericAtomKind) -> Option<f32> {
        use NumericAtomKind as K;
        match (self, kind) {
            (Self::Linear(axis), K::GradientAngle) => Some(axis.heading().0),
            (Self::Conic(conic), K::GradientAngle) => Some(f32::from(conic.from)),
            (
                Self::Radial(Radial { at: [at, _], .. }) | Self::Conic(Conic { at: [at, _], .. }),
                K::GradientCenterX,
            )
            | (
                Self::Radial(Radial { at: [_, at], .. }) | Self::Conic(Conic { at: [_, at], .. }),
                K::GradientCenterY,
            ) => Some(f32::from(at)),
            _ => None,
        }
    }
}

impl StopSlot {
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::From => 0,
            Self::Via => 1,
            Self::To => 2,
        }
    }

    pub(crate) fn default_at(self) -> u8 {
        PaintStyle::default().at(self.index())
    }
}

const STOPS: [(NumericAtomKind, ColorAtomKind); 3] = [
    (NumericAtomKind::StopFrom, ColorAtomKind::StopFrom),
    (NumericAtomKind::StopVia, ColorAtomKind::StopVia),
    (NumericAtomKind::StopTo, ColorAtomKind::StopTo),
];

const CLEAR: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 0,
};

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the percent is clamped to 0..=100 before the cast"
)]
const fn percent_units(value: f32) -> u8 {
    value.round().clamp(0.0, 100.0) as u8
}

impl PaintStyle {
    const fn via(&self) -> bool {
        matches!(self.stops, [_, Some(_), _])
    }

    const fn drawn(&self, index: usize) -> bool {
        self.gradient.is_some() && (index != 1 || self.via())
    }

    pub(crate) fn value(&self, kind: NumericAtomKind) -> Option<f32> {
        let Some(index) = STOPS.iter().position(|(owner, _)| *owner == kind) else {
            return self.gradient.and_then(|shape| shape.value(kind));
        };
        let position = self.positions.get(index).copied().flatten();
        self.drawn(index)
            .then(|| position.map_or_else(|| kind.initial(), f32::from))
    }

    pub(crate) fn color(&self, kind: ColorAtomKind) -> Option<Color> {
        let index = STOPS.iter().position(|(_, owner)| *owner == kind)?;
        let stop = self.stops.get(index).copied().flatten();
        self.drawn(index).then(|| stop.unwrap_or(CLEAR))
    }

    pub(crate) fn at(&self, index: usize) -> u8 {
        let set = self.positions.get(index).copied().flatten();
        let default = STOPS
            .get(index)
            .map(|(kind, _)| percent_units(kind.initial()));
        set.or(default).unwrap_or_default()
    }

    pub(crate) const fn shape(&self) -> Option<(GradientShape, bool)> {
        match self.gradient {
            Some(shape) => Some((shape, self.via())),
            None => None,
        }
    }

    pub(crate) fn overlay(&mut self, overlay: &dyn AnimatorOverlay) {
        let shown = [0, 1, 2].map(|index| self.drawn(index));
        let slots = self.stops.iter_mut().zip(self.positions.iter_mut());
        for ((stop, position), ((numeric, color), drawn)) in slots.zip(STOPS.into_iter().zip(shown))
        {
            if !drawn {
                continue;
            }
            if let Some(color) = overlay.value_for_color(color) {
                *stop = Some(color);
            }
            if let Some(percent) = overlay.value_for(numeric) {
                *position = Some(percent_units(percent));
            }
        }
        if let Some(shape) = self.gradient {
            self.geometry.overlay(shape, overlay);
        }
    }
}
