use std::num::NonZeroU32;
use std::time::Duration;

use super::Easing;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Keyframe {
    pub(crate) offset: f32,
    pub(crate) value: f32,
    pub(crate) easing: Easing,
}

impl Keyframe {
    #[must_use]
    pub fn new(offset: f32, value: f32) -> Self {
        assert!((0.0..=1.0).contains(&offset) && value.is_finite());
        Self {
            offset,
            value,
            easing: Easing::Linear,
        }
    }

    #[must_use]
    pub const fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnimationDirection {
    #[default]
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnimationFill {
    None,
    #[default]
    Forwards,
    Backwards,
    Both,
}

#[derive(Clone, Debug)]
pub struct Keyframes {
    frames: Box<[Keyframe]>,
    pub(crate) duration: Duration,
    pub(crate) delay: Duration,
    pub(crate) iterations: Option<NonZeroU32>,
    pub(crate) direction: AnimationDirection,
    pub(crate) fill: AnimationFill,
}

impl Keyframes {
    #[must_use]
    pub fn new(frames: impl IntoIterator<Item = Keyframe>, duration: Duration) -> Self {
        let frames: Box<[Keyframe]> = frames.into_iter().collect();
        assert!(frames.len() >= 2 && !duration.is_zero());
        assert!(
            frames.first().is_some_and(|frame| frame.offset == 0.0)
                && frames
                    .last()
                    .is_some_and(|frame| frame.offset.to_bits() == 1.0_f32.to_bits())
        );
        assert!(
            frames
                .windows(2)
                .all(|pair| matches!(pair, [from, to] if from.offset < to.offset))
        );
        Self {
            frames,
            duration,
            delay: Duration::ZERO,
            iterations: NonZeroU32::new(1),
            direction: AnimationDirection::Normal,
            fill: AnimationFill::Forwards,
        }
    }

    #[must_use]
    pub const fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    #[must_use]
    pub const fn repeat(mut self, iterations: NonZeroU32) -> Self {
        self.iterations = Some(iterations);
        self
    }

    #[must_use]
    pub const fn repeat_forever(mut self) -> Self {
        self.iterations = None;
        self
    }

    #[must_use]
    pub const fn direction(mut self, direction: AnimationDirection) -> Self {
        self.direction = direction;
        self
    }

    #[must_use]
    pub const fn fill(mut self, fill: AnimationFill) -> Self {
        self.fill = fill;
        self
    }

    #[must_use]
    pub fn sample(&self, progress: f32) -> f32 {
        debug_assert!(progress.is_finite(), "keyframe progress must be finite");
        let progress = if progress.is_nan() {
            0.0
        } else {
            progress.clamp(0.0, 1.0)
        };
        let index = self
            .frames
            .partition_point(|frame| frame.offset <= progress);
        let Some(to) = self.frames.get(index) else {
            return self.frames.last().expect("validated endpoints").value;
        };
        let from = self
            .frames
            .get(index - 1)
            .expect("progress is at or after the first keyframe");
        let fraction = (progress - from.offset) / (to.offset - from.offset);
        (to.value - from.value).mul_add(from.easing.apply(fraction), from.value)
    }
}

impl Keyframes {
    pub(crate) fn at(&self, seconds: f64, initial: f32) -> Option<f32> {
        let local = seconds - self.delay.as_secs_f64();
        if local < 0.0 {
            return Some(
                if matches!(self.fill, AnimationFill::Backwards | AnimationFill::Both) {
                    self.sample_cycle(0.0, 0.0)
                } else {
                    initial
                },
            );
        }
        let cycles = local / self.duration.as_secs_f64();
        if let Some(count) = self.iterations
            && cycles >= f64::from(count.get())
        {
            return None;
        }
        Some(self.sample_cycle(cycles.floor(), fraction(cycles.fract())))
    }

    pub(crate) fn end_value(&self, initial: f32) -> f32 {
        let last = self
            .iterations
            .map_or(0.0, |count| f64::from(count.get() - 1));
        if matches!(self.fill, AnimationFill::Forwards | AnimationFill::Both) {
            self.sample_cycle(last, 1.0)
        } else {
            initial
        }
    }

    fn sample_cycle(&self, iteration: f64, progress: f32) -> f32 {
        let reverse = match self.direction {
            AnimationDirection::Normal => false,
            AnimationDirection::Reverse => true,
            AnimationDirection::Alternate => iteration.rem_euclid(2.0) >= 1.0,
            AnimationDirection::AlternateReverse => iteration.rem_euclid(2.0) < 1.0,
        };
        self.sample(if reverse { 1.0 - progress } else { progress })
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "fraction is bounded to 0..1"
)]
const fn fraction(value: f64) -> f32 {
    value as f32
}
