use core::f32::consts::TAU;
use std::hash::{Hash, Hasher};
use std::time::Duration;

use super::Easing;
use crate::style::{ColorAtom, ColorAtomKind};

mod property;
mod sample;
mod store;

pub use property::Property;
kittest_pub! {
    pub use store::KeyframesId;
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
    frames: Vec<(f32, Property)>,
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
            frames: Vec::new(),
            easings: Vec::new(),
        }
    }

    /// Sets `property` at `offset`, from 0.0 to 1.0; a layout property is ignored with a warning.
    pub fn keyframe(mut self, offset: f32, property: Property) -> Self {
        if property.layout() {
            property.refuse("Keyframes::keyframe");
        } else {
            self.frames.push((unit(offset), property));
        }
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
        self.frames.iter().flat_map(|(_, property)| {
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
    const fn timing(&self) -> (Duration, Easing, Option<u32>, bool, bool) {
        (
            self.duration,
            self.easing,
            self.iterations,
            self.reverse,
            self.alternate,
        )
    }

    fn stop_keys(&self) -> impl Iterator<Item = (u32, property::Key)> + '_ {
        self.frames
            .iter()
            .map(|(offset, property)| (offset.to_bits(), property.key()))
    }

    fn easing_keys(&self) -> impl Iterator<Item = (u32, Easing)> + '_ {
        self.easings
            .iter()
            .map(|(offset, easing)| (offset.to_bits(), *easing))
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
