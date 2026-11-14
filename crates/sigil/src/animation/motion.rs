use std::num::NonZeroU16;

use super::runtime::AnimationConfig;
use super::{Decay, Easing, SpringState, Tween};

const VELOCITY_PROBE_SECS: f32 = 0.001;

pub(crate) enum Motion {
    Spring(SpringState),
    Tween {
        current: f32,
        secs: f32,
        delay: f32,
        easing: Easing,
        tween: Option<Tween>,
    },
    Playback {
        elapsed: f64,
        end: Option<f64>,
        cycle: f64,
    },
    Decay(Decay),
}

impl Motion {
    #[cfg(test)]
    pub(crate) fn spring(initial: f32, spring: super::Spring) -> Self {
        Self::Spring(SpringState::new(initial, spring))
    }

    pub(crate) const fn tween(initial: f32, ms: u16, easing: Easing, delay_ms: u16) -> Self {
        Self::Tween {
            current: initial,
            secs: ms as f32 / 1000.0,
            delay: delay_ms as f32 / 1000.0,
            easing,
            tween: None,
        }
    }

    pub(crate) const fn playback(end: Option<f64>, cycle: f64, delay: f64) -> Self {
        Self::Playback {
            elapsed: -delay,
            end,
            cycle,
        }
    }

    pub(crate) fn configured(initial: f32, velocity: f32, config: AnimationConfig) -> Self {
        match config {
            AnimationConfig::Spring(spring) => {
                let mut state = SpringState::new(initial, spring);
                state.set_velocity(velocity);
                Self::Spring(state)
            }
            AnimationConfig::Tween {
                ms,
                easing,
                delay_ms,
            } => Self::tween(initial, ms, easing, delay_ms),
            AnimationConfig::Playback { end, cycle, delay } => Self::playback(end, cycle, delay),
        }
    }

    pub(crate) fn reconfigure(&mut self, config: AnimationConfig) {
        let settled = self.is_settled();
        let target = self.target();
        *self = Self::configured(self.value(), self.velocity(), config);
        if !settled {
            self.retarget(target);
        }
    }

    pub(crate) fn resume(&mut self, config: AnimationConfig) {
        if matches!(self, Self::Decay(_)) {
            self.reconfigure(config);
        }
    }

    pub(crate) fn fling(
        &mut self,
        velocity: f32,
        points: &[f32],
        config: AnimationConfig,
    ) -> Option<f32> {
        let value = self.value();
        let coast = Decay::new(value, velocity);
        let rest = coast.target();
        let nearest = points
            .iter()
            .copied()
            .filter(|point| point.is_finite())
            .min_by(|a, b| (a - rest).abs().total_cmp(&(b - rest).abs()));
        *self = match nearest {
            Some(point) => {
                let mut snap = Self::configured(value, velocity, config);
                snap.retarget(point);
                snap
            }
            None => Self::Decay(coast),
        };
        nearest
    }

    pub(crate) fn tick(&mut self, dt: f32) {
        match self {
            Self::Spring(spring) => spring.advance(dt),
            Self::Tween { current, tween, .. } => step_tween(current, tween, dt),
            Self::Playback { elapsed, end, .. } => {
                *elapsed += f64::from(dt);
                if let Some(end) = *end {
                    *elapsed = elapsed.min(end);
                }
            }
            Self::Decay(decay) => decay.step(dt),
        }
    }

    pub(crate) fn retarget(&mut self, value: f32) {
        match self {
            Self::Spring(spring) => spring.set_target(value),
            Self::Tween {
                tween: Some(active),
                ..
            } if active.to().to_bits() == value.to_bits() => {}
            Self::Tween {
                current,
                secs,
                delay,
                easing,
                tween,
            } => {
                *tween =
                    if (*secs <= 0.0 && *delay <= 0.0) || (*current - value).abs() < f32::EPSILON {
                        *current = value;
                        None
                    } else {
                        Some(timed(*current, value, [*secs, *delay], *easing))
                    };
            }
            Self::Playback { .. } | Self::Decay(_) => {}
        }
    }

    pub(crate) const fn finish(&mut self) {
        match self {
            Self::Spring(spring) => spring.reset(spring.target()),
            Self::Tween { current, tween, .. } => {
                if let Some(active) = tween.take() {
                    *current = active.to();
                }
            }
            Self::Playback { elapsed, end, .. } => {
                if let Some(end) = *end {
                    *elapsed = end;
                }
            }
            Self::Decay(decay) => decay.finish(),
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "the phase is below two passes, so f32 keeps sub-millisecond precision"
    )]
    pub(crate) fn value(&self) -> f32 {
        match self {
            Self::Spring(spring) => spring.value(),
            Self::Tween { current, .. } => *current,
            Self::Playback { elapsed, cycle, .. } if *cycle > 0.0 => (*elapsed % *cycle) as f32,
            Self::Playback { .. } => 0.0,
            Self::Decay(decay) => decay.value(),
        }
    }

    pub(crate) fn velocity(&self) -> f32 {
        match self {
            Self::Spring(spring) => spring.velocity(),
            Self::Tween {
                tween: Some(active),
                ..
            } => {
                let mut ahead = *active;
                ahead.step(VELOCITY_PROBE_SECS);
                (ahead.value() - active.value()) / VELOCITY_PROBE_SECS
            }
            Self::Decay(decay) => decay.velocity(),
            Self::Tween { .. } | Self::Playback { .. } => 0.0,
        }
    }

    fn target(&self) -> f32 {
        match self {
            Self::Spring(spring) => spring.target(),
            Self::Tween { current, tween, .. } => tween.map_or(*current, |active| active.to()),
            Self::Decay(decay) => decay.target(),
            Self::Playback { .. } => self.value(),
        }
    }

    pub(crate) fn is_settled(&self) -> bool {
        match self {
            Self::Spring(spring) => spring.is_settled(),
            Self::Tween { tween, .. } => tween.is_none(),
            Self::Playback { elapsed, end, .. } => end.is_some_and(|end| *elapsed >= end),
            Self::Decay(decay) => decay.is_settled(),
        }
    }

    pub(crate) const fn is_perpetual(&self) -> bool {
        matches!(self, Self::Playback { end: None, .. })
    }
}

fn timed(from: f32, to: f32, [secs, delay]: [f32; 2], easing: Easing) -> Tween {
    if secs > 0.0 {
        return Tween::new(from, to, secs)
            .with_easing(easing)
            .with_delay(delay);
    }
    Tween::new(from, to, delay).with_easing(Easing::steps(NonZeroU16::MIN, false))
}

fn step_tween(current: &mut f32, tween: &mut Option<Tween>, dt: f32) {
    let Some(active) = tween.as_mut() else {
        return;
    };
    active.step(dt);
    *current = active.value();
    if active.is_finished() {
        *tween = None;
    }
}
