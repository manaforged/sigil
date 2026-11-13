use smallvec::{SmallVec, smallvec};

use super::gradient::percent as percent_of;
use super::{Color, ColorValue, Conic, GradientAxis, GradientShape, PaintAtom, Radial, StopSlot};
use crate::style::{StyleAtom, Token};

type Atoms = SmallVec<[StyleAtom; 8]>;

pub(crate) const NO_GRADIENT: StyleAtom = StyleAtom::Paint(PaintAtom::NoGradient);

/// What fills a node's background: one color, or one gradient.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub struct Background(Fill);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Fill {
    Color(ColorValue),
    Gradient(Gradient),
}

/// A linear, radial, or conic gradient with up to three color stops, made by `Background::linear`, `radial`, or `conic`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub struct Gradient {
    shape: GradientShape,
    over: Option<ColorValue>,
    start: Stop,
    middle: Stop,
    end: Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Stop {
    color: Option<ColorValue>,
    at: Option<u8>,
}

impl Background {
    /// A linear gradient along `axis`, like Tailwind `bg-linear-to-*`.
    pub const fn linear(axis: GradientAxis) -> Gradient {
        Gradient::shaped(GradientShape::Linear(axis))
    }

    /// A radial gradient, like Tailwind `bg-radial`.
    pub const fn radial(radial: Radial) -> Gradient {
        Gradient::shaped(GradientShape::Radial(radial))
    }

    /// A conic gradient, like Tailwind `bg-conic-*`.
    pub fn conic(conic: impl Into<Conic>) -> Gradient {
        Gradient::shaped(GradientShape::Conic(conic.into()))
    }

    pub(crate) fn atoms(self) -> Atoms {
        match self.0 {
            Fill::Color(color) => smallvec![StyleAtom::BackgroundColor(color), NO_GRADIENT],
            Fill::Gradient(gradient) => gradient.atoms(),
        }
    }
}

impl Gradient {
    /// Sets the first gradient color, like Tailwind `from-*`.
    pub fn start(mut self, color: impl Into<ColorValue>) -> Self {
        self.start.color = Some(color.into());
        self
    }

    /// Sets the middle gradient color, like Tailwind `via-*`.
    pub fn middle(mut self, color: impl Into<ColorValue>) -> Self {
        self.middle.color = Some(color.into());
        self
    }

    /// Sets the last gradient color, like Tailwind `to-*`.
    pub fn end(mut self, color: impl Into<ColorValue>) -> Self {
        self.end.color = Some(color.into());
        self
    }

    /// Sets where the first color sits in percent, like Tailwind `from-<n>%`; default 0.
    pub const fn start_at(mut self, percent: u8) -> Self {
        self.start.at = Some(percent_of(percent));
        self
    }

    /// Sets where the middle color sits in percent, like Tailwind `via-<n>%`; default 50.
    pub const fn middle_at(mut self, percent: u8) -> Self {
        self.middle.at = Some(percent_of(percent));
        self
    }

    /// Sets where the last color sits in percent, like Tailwind `to-<n>%`; default 100.
    pub const fn end_at(mut self, percent: u8) -> Self {
        self.end.at = Some(percent_of(percent));
        self
    }

    /// Sets the color painted under the gradient, like CSS `background-color`; default none.
    pub fn over(mut self, color: impl Into<ColorValue>) -> Self {
        self.over = Some(color.into());
        self
    }

    const fn shaped(shape: GradientShape) -> Self {
        let unset = Stop {
            color: None,
            at: None,
        };
        Self {
            shape,
            over: None,
            start: unset,
            middle: unset,
            end: unset,
        }
    }

    fn atoms(self) -> Atoms {
        let under = self.over.map_or(
            StyleAtom::Paint(PaintAtom::NoColor),
            StyleAtom::BackgroundColor,
        );
        let shape = StyleAtom::Paint(PaintAtom::Gradient(self.shape));
        let stops = [
            (StopSlot::From, self.start),
            (StopSlot::Via, self.middle),
            (StopSlot::To, self.end),
        ]
        .into_iter()
        .flat_map(|(slot, stop)| {
            [
                stop.color.map_or(PaintAtom::NoStop(slot), |color| {
                    PaintAtom::Stop(slot, color)
                }),
                PaintAtom::StopAt(slot, stop.at.unwrap_or_else(|| slot.default_at())),
            ]
        })
        .map(StyleAtom::Paint);
        [under, shape].into_iter().chain(stops).collect()
    }
}

impl From<ColorValue> for Background {
    fn from(color: ColorValue) -> Self {
        Self(Fill::Color(color))
    }
}

impl From<Color> for Background {
    fn from(color: Color) -> Self {
        Self(Fill::Color(color.into()))
    }
}

impl From<Token> for Background {
    fn from(token: Token) -> Self {
        Self(Fill::Color(token.into()))
    }
}

impl From<Gradient> for Background {
    fn from(gradient: Gradient) -> Self {
        Self(Fill::Gradient(gradient))
    }
}
