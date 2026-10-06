use web_time::Instant;

use crate::animation_runtime::active_count as active_animation_count;
#[cfg(test)]
use crate::animation_runtime::tick as tick_animations;

const NOMINAL_FRAME_SECS: f32 = 1.0 / 60.0;

#[derive(Default)]
pub(crate) struct AnimationClock {
    pub(super) previous: Option<Instant>,
}

impl AnimationClock {
    pub(crate) fn elapsed(&mut self, now: Instant) -> f32 {
        if active_animation_count() == 0 {
            self.previous = None;
            return NOMINAL_FRAME_SECS;
        }
        self.previous
            .replace(now)
            .map_or(NOMINAL_FRAME_SECS, |previous| {
                now.saturating_duration_since(previous).as_secs_f32()
            })
    }

    #[cfg(test)]
    pub(crate) fn tick_active(&mut self, now: Instant) -> bool {
        if active_animation_count() == 0 {
            self.previous = None;
            return false;
        }
        let dt = self.elapsed(now);
        tick_animations(dt);
        if active_animation_count() == 0 {
            self.previous = None;
        }
        true
    }
}
