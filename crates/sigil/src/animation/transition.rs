use std::time::Duration;

use super::Easing;
use crate::animation::runtime::presets;
use crate::animation::runtime::registry::timing;
use crate::style::{MillisU16, StyleAtom, TransitionId};

/// The properties a node animates when its style changes, with the duration, easing, and delay.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[must_use]
pub struct Transition {
    id: TransitionId,
    duration: Option<MillisU16>,
    easing: Option<Easing>,
    delay: Option<MillisU16>,
}

impl Transition {
    /// Transitions colors, opacity, and transform over 150 ms, like Tailwind `transition`.
    pub const fn new() -> Self {
        Self::preset(presets::TRANSITION)
    }

    /// Transitions every animatable property over 150 ms, like Tailwind `transition-all`.
    pub const fn all() -> Self {
        Self::preset(presets::ALL)
    }

    /// Transitions background, text, and border colors, like Tailwind `transition-colors`.
    pub const fn colors() -> Self {
        Self::preset(presets::COLORS)
    }

    /// Transitions opacity, like Tailwind `transition-opacity`.
    pub const fn opacity() -> Self {
        Self::preset(presets::OPACITY)
    }

    /// Transitions translate, scale, and rotate, like Tailwind `transition-transform`.
    pub const fn transform() -> Self {
        Self::preset(presets::TRANSFORM)
    }

    /// Sets how long the change takes, like Tailwind `duration-*`.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(millis(duration));
        self
    }

    /// Sets the easing of the change, like Tailwind `ease-*`.
    pub const fn ease(mut self, easing: Easing) -> Self {
        self.easing = Some(easing);
        self
    }

    /// Holds the start value for `delay` before the change moves, like Tailwind `delay-*`.
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = Some(millis(delay));
        self
    }

    pub(crate) fn atoms(self) -> [StyleAtom; 4] {
        let (ms, easing, delay_ms) = timing(self.id);
        [
            StyleAtom::Transition(self.id),
            StyleAtom::TransitionDuration(self.duration.unwrap_or(MillisU16(ms))),
            StyleAtom::TransitionEase(self.easing.unwrap_or(easing)),
            StyleAtom::TransitionDelay(self.delay.unwrap_or(MillisU16(delay_ms))),
        ]
    }

    const fn preset(id: TransitionId) -> Self {
        Self {
            id,
            duration: None,
            easing: None,
            delay: None,
        }
    }
}

impl Default for Transition {
    fn default() -> Self {
        Self::new()
    }
}

impl From<TransitionId> for Transition {
    fn from(id: TransitionId) -> Self {
        Self::preset(id)
    }
}

fn millis(duration: Duration) -> MillisU16 {
    MillisU16(u16::try_from(duration.as_millis()).unwrap_or(u16::MAX))
}
