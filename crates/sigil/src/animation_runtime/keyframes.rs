use crate::animation::{AnimationDirection, AnimationFill, Keyframes};

use super::AnimatorStatus;

pub(crate) struct KeyframeMotion {
    frames: Keyframes,
    elapsed: f64,
    initial: f32,
    current: f32,
    finished: bool,
}

impl KeyframeMotion {
    pub(crate) fn new(frames: Keyframes, initial: f32) -> Self {
        let mut motion = Self {
            frames,
            elapsed: 0.0,
            initial,
            current: initial,
            finished: false,
        };
        motion.seek(0.0);
        motion
    }

    pub(crate) fn tick(&mut self, dt: f32) -> AnimatorStatus {
        self.seek(self.elapsed + f64::from(dt));
        if self.finished {
            AnimatorStatus::Settled
        } else {
            AnimatorStatus::Running
        }
    }

    pub(crate) fn seek(&mut self, seconds: f64) {
        self.elapsed = seconds;
        let local = seconds - self.frames.delay.as_secs_f64();
        if local < 0.0 {
            self.finished = false;
            self.current = if matches!(
                self.frames.fill,
                AnimationFill::Backwards | AnimationFill::Both
            ) {
                self.sample(0.0, 0.0)
            } else {
                self.initial
            };
            return;
        }
        let cycles = local / self.frames.duration.as_secs_f64();
        if let Some(count) = self.frames.iterations
            && cycles >= f64::from(count.get())
        {
            self.finish();
            return;
        }
        self.finished = false;
        self.current = self.sample(cycles.floor(), fraction(cycles.fract()));
    }

    fn sample(&self, iteration: f64, progress: f32) -> f32 {
        let reverse = match self.frames.direction {
            AnimationDirection::Normal => false,
            AnimationDirection::Reverse => true,
            AnimationDirection::Alternate => iteration.rem_euclid(2.0) >= 1.0,
            AnimationDirection::AlternateReverse => iteration.rem_euclid(2.0) < 1.0,
        };
        self.frames
            .sample(if reverse { 1.0 - progress } else { progress })
    }

    pub(crate) fn finish(&mut self) {
        let last = self
            .frames
            .iterations
            .map_or(0.0, |count| f64::from(count.get() - 1));
        self.current = if matches!(
            self.frames.fill,
            AnimationFill::Forwards | AnimationFill::Both
        ) {
            self.sample(last, 1.0)
        } else {
            self.initial
        };
        self.finished = true;
    }

    pub(crate) const fn cancel(&mut self) {
        self.current = self.initial;
        self.finished = true;
    }

    pub(crate) const fn value(&self) -> f32 {
        self.current
    }
    pub(crate) const fn is_settled(&self) -> bool {
        self.finished
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "fraction is bounded to 0..1"
)]
const fn fraction(value: f64) -> f32 {
    value as f32
}
