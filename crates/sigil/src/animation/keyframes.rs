use core::f32::consts::TAU;
use std::hash::{Hash, Hasher};
use std::time::Duration;

use super::Easing;
use crate::style::{ColorAtom, ColorAtomKind, NumericAtomKind};

mod sample;
mod store;

pub use store::KeyframesId;

/// One property value that a keyframe sets.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Property {
    /// Opacity, from 0.0 to 1.0.
    Opacity(f32),
    /// Horizontal translation in pixels.
    TranslateX(f32),
    /// Vertical translation in pixels.
    TranslateY(f32),
    /// Horizontal translation in percent of the node's width.
    TranslateXPercent(f32),
    /// Vertical translation in percent of the node's height.
    TranslateYPercent(f32),
    /// Uniform scale factor.
    Scale(f32),
    /// Rotation in radians.
    Rotate(f32),
    /// Rotation around the x axis in radians.
    RotateX(f32),
    /// Rotation around the y axis in radians.
    RotateY(f32),
    /// Translation toward the viewer in pixels.
    TranslateZ(f32),
    /// Background color.
    Background(ColorAtom),
    /// Text color.
    TextColor(ColorAtom),
    /// Border color.
    BorderColor(ColorAtom),
}

const OPACITY_UNITS: f32 = 10_000.0;

impl Property {
    pub(crate) fn numeric(self, kind: NumericAtomKind) -> Option<f32> {
        match (self, kind) {
            (Self::Opacity(value), NumericAtomKind::Opacity) => {
                Some(value.clamp(0.0, 1.0) * OPACITY_UNITS)
            }
            (Self::TranslateX(px), NumericAtomKind::TranslateX)
            | (Self::TranslateY(px), NumericAtomKind::TranslateY)
            | (Self::Rotate(px), NumericAtomKind::Rotation)
            | (Self::RotateX(px), NumericAtomKind::RotationX)
            | (Self::RotateY(px), NumericAtomKind::RotationY)
            | (Self::TranslateZ(px), NumericAtomKind::TranslateZ)
            | (Self::Scale(px), NumericAtomKind::ScaleX | NumericAtomKind::ScaleY) => Some(px),
            (Self::TranslateXPercent(percent), NumericAtomKind::TranslateXFraction)
            | (Self::TranslateYPercent(percent), NumericAtomKind::TranslateYFraction) => {
                Some(percent / 100.0)
            }
            _ => None,
        }
        .filter(|value| value.is_finite())
    }

    pub(crate) const fn color(self, kind: ColorAtomKind) -> Option<ColorAtom> {
        match (self, kind) {
            (Self::Background(color), ColorAtomKind::Background)
            | (Self::TextColor(color), ColorAtomKind::Text)
            | (Self::BorderColor(color), ColorAtomKind::Border) => Some(color),
            _ => None,
        }
    }

    fn key(self) -> (u8, u32, Option<ColorAtom>) {
        match self {
            Self::Opacity(value) => (0, bits(value), None),
            Self::TranslateX(value) => (1, bits(value), None),
            Self::TranslateY(value) => (2, bits(value), None),
            Self::TranslateXPercent(value) => (3, bits(value), None),
            Self::TranslateYPercent(value) => (4, bits(value), None),
            Self::Scale(value) => (5, bits(value), None),
            Self::Rotate(value) => (6, bits(value), None),
            Self::Background(color) => (7, 0, Some(color)),
            Self::TextColor(color) => (8, 0, Some(color)),
            Self::BorderColor(color) => (9, 0, Some(color)),
            Self::RotateX(value) => (10, bits(value), None),
            Self::RotateY(value) => (11, bits(value), None),
            Self::TranslateZ(value) => (12, bits(value), None),
        }
    }
}

fn bits(value: f32) -> u32 {
    if value.is_nan() {
        f32::NAN.to_bits()
    } else {
        (value + 0.0).to_bits()
    }
}

/// A CSS keyframe animation: property values at offsets of one pass, its timing, and its repeats.
#[derive(Clone, Debug)]
#[must_use]
pub struct Keyframes {
    duration: Duration,
    easing: Easing,
    iterations: Option<u32>,
    reverse: bool,
    alternate: bool,
    keyframes: Vec<(f32, Property)>,
    easings: Vec<(f32, Easing)>,
}

const CSS_EASE: Easing = Easing::cubic_bezier(0.25, 0.1, 0.25, 1.0);

impl Keyframes {
    /// Creates an animation with one pass of `duration`, the CSS `ease` curve, and no keyframes.
    pub const fn new(duration: Duration) -> Self {
        Self {
            duration,
            easing: CSS_EASE,
            iterations: Some(1),
            reverse: false,
            alternate: false,
            keyframes: Vec::new(),
            easings: Vec::new(),
        }
    }

    /// Sets `property` at `offset`, from 0.0 (start of a pass) to 1.0 (end).
    pub fn keyframe(mut self, offset: f32, property: Property) -> Self {
        self.keyframes.push((unit(offset), property));
        self
    }

    /// Sets the easing between each pair of keyframes, like `animation-timing-function`.
    pub const fn ease(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// Sets the easing from the keyframes at `offset` to the next ones.
    pub fn ease_at(mut self, offset: f32, easing: Easing) -> Self {
        self.easings.push((unit(offset), easing));
        self
    }

    /// Sets the number of passes; the default is 1.
    pub const fn iterations(mut self, count: u32) -> Self {
        self.iterations = Some(count);
        self
    }

    /// Repeats the passes forever, like `animation-iteration-count: infinite`.
    pub const fn infinite(mut self) -> Self {
        self.iterations = None;
        self
    }

    /// Plays each pass from 1.0 to 0.0, like `animation-direction: reverse`.
    pub const fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    /// Plays every second pass the other way, like `animation-direction: alternate`.
    pub const fn alternate(mut self) -> Self {
        self.alternate = true;
        self
    }

    /// The Tailwind `animate-spin` animation.
    pub fn spin() -> Self {
        Self::new(Duration::from_secs(1))
            .ease(Easing::Linear)
            .keyframe(1.0, Property::Rotate(TAU))
            .infinite()
    }

    /// The Tailwind `animate-ping` animation.
    pub fn ping() -> Self {
        Self::new(Duration::from_secs(1))
            .ease(Easing::OUT)
            .keyframe(0.75, Property::Scale(2.0))
            .keyframe(0.75, Property::Opacity(0.0))
            .keyframe(1.0, Property::Scale(2.0))
            .keyframe(1.0, Property::Opacity(0.0))
            .infinite()
    }

    /// The Tailwind `animate-pulse` animation.
    pub fn pulse() -> Self {
        Self::new(Duration::from_secs(2))
            .ease(Easing::cubic_bezier(0.4, 0.0, 0.6, 1.0))
            .keyframe(0.5, Property::Opacity(0.5))
            .infinite()
    }

    /// The Tailwind `animate-bounce` animation.
    pub fn bounce() -> Self {
        let fall = Easing::cubic_bezier(0.8, 0.0, 1.0, 1.0);
        Self::new(Duration::from_secs(1))
            .keyframe(0.0, Property::TranslateYPercent(-25.0))
            .keyframe(0.5, Property::TranslateYPercent(0.0))
            .keyframe(1.0, Property::TranslateYPercent(-25.0))
            .ease_at(0.0, fall)
            .ease_at(0.5, Easing::OUT)
            .ease_at(1.0, fall)
            .infinite()
    }

    pub(crate) fn end(&self) -> Option<f64> {
        let pass = self.duration.as_secs_f64();
        if pass <= 0.0 {
            return Some(0.0);
        }
        self.iterations.map(|count| f64::from(count) * pass)
    }

    pub(crate) fn cycle(&self) -> f64 {
        2.0 * self.duration.as_secs_f64()
    }

    pub(crate) fn progress(&self, elapsed: f32) -> Option<f32> {
        let duration = self.duration.as_secs_f32();
        if duration <= 0.0 {
            return None;
        }
        let passes = elapsed.max(0.0) / duration;
        let pass = passes.floor();
        let odd = pass % 2.0 >= 1.0;
        let backward = self.reverse != (self.alternate && odd);
        let local = passes - pass;
        Some(if backward { 1.0 - local } else { local })
    }

    pub(crate) fn colors(&self) -> impl Iterator<Item = ColorAtom> + '_ {
        self.keyframes.iter().flat_map(|(_, property)| {
            ColorAtomKind::ALL
                .into_iter()
                .filter_map(move |kind| property.color(kind))
        })
    }

    fn easing_at(&self, offset: f32) -> Easing {
        self.easings
            .iter()
            .rev()
            .find(|(at, _)| at.to_bits() == offset.to_bits())
            .map_or(self.easing, |(_, easing)| *easing)
    }
}

fn unit(offset: f32) -> f32 {
    if offset.is_finite() {
        offset.clamp(0.0, 1.0) + 0.0
    } else {
        0.0
    }
}

impl Keyframes {
    fn timing(&self) -> (Duration, Easing, Option<u32>, bool, bool) {
        (self.duration, self.easing, self.iterations, self.reverse, self.alternate)
    }

    fn stop_keys(&self) -> impl Iterator<Item = (u32, (u8, u32, Option<ColorAtom>))> + '_ {
        self.keyframes
            .iter()
            .map(|(offset, property)| (offset.to_bits(), property.key()))
    }

    fn easing_keys(&self) -> impl Iterator<Item = (u32, Easing)> + '_ {
        self.easings.iter().map(|(offset, easing)| (offset.to_bits(), *easing))
    }
}

impl PartialEq for Keyframes {
    fn eq(&self, other: &Self) -> bool {
        self.timing() == other.timing()
            && self.stop_keys().eq(other.stop_keys())
            && self.easing_keys().eq(other.easing_keys())
    }
}

impl Eq for Keyframes {}

impl Hash for Keyframes {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.timing().hash(state);
        self.stop_keys().for_each(|key| key.hash(state));
        self.easing_keys().for_each(|key| key.hash(state));
    }
}
