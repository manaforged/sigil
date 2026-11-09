use super::{ColorAtom, GradientAxis, Rgba8};
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

/// The line style of a border.
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

/// A paint style atom: gradients, stops, clip, image, and border style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaintAtom {
    /// The background gradient geometry.
    Gradient(GradientShape),
    /// No background gradient.
    NoGradient,
    /// A gradient stop color.
    Stop(StopSlot, ColorAtom),
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
    /// The border line style.
    Border(BorderStyle),
}

/// The resolved paint atoms of one node.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PaintStyle {
    /// The background gradient geometry, if any.
    pub gradient: Option<GradientShape>,
    /// The from, via, and to colors.
    pub stops: [Option<Rgba8>; 3],
    /// The from, via, and to positions in percent.
    pub positions: [Option<u8>; 3],
    /// Whether the background shows only inside text.
    pub clip_text: bool,
    /// The tiled background image id, if any.
    pub image: Option<u64>,
    /// The border line style.
    pub border: BorderStyle,
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

const CLEAR: Rgba8 = Rgba8 {
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
        let index = STOPS.iter().position(|(owner, _)| *owner == kind)?;
        let position = self.positions.get(index).copied().flatten();
        self.drawn(index)
            .then(|| position.map_or_else(|| kind.initial(), f32::from))
    }

    pub(crate) fn color(&self, kind: ColorAtomKind) -> Option<Rgba8> {
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
    }
}
