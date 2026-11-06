use super::{Easing, Spring, SpringPreset, Tween};

pub(crate) enum Motion {
    Spring(Spring),
    Tween {
        current: f32,
        secs: f32,
        delay: f32,
        easing: Easing,
        tween: Option<Tween>,
    },
    Playback {
        elapsed: f32,
        end: Option<f32>,
    },
}

impl Motion {
    pub(crate) fn spring(initial: f32, preset: SpringPreset) -> Self {
        Self::Spring(Spring::from_preset(initial, preset))
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

    pub(crate) const fn playback(end: Option<f32>) -> Self {
        Self::Playback { elapsed: 0.0, end }
    }

    pub(crate) fn tick(&mut self, dt: f32) {
        match self {
            Self::Spring(spring) => spring.advance(dt),
            Self::Tween { current, tween, .. } => step_tween(current, tween, dt),
            Self::Playback { elapsed, end } => {
                *elapsed += dt;
                if let Some(end) = *end {
                    *elapsed = elapsed.min(end);
                }
            }
        }
    }

    pub(crate) fn retarget(&mut self, value: f32) {
        match self {
            Self::Spring(spring) => spring.set_target(value),
            Self::Tween {
                current,
                secs,
                delay,
                easing,
                tween,
            } => {
                *tween = if *secs <= 0.0 {
                    *current = value;
                    None
                } else if (*current - value).abs() < f32::EPSILON {
                    None
                } else {
                    Some(
                        Tween::new(*current, value, *secs)
                            .with_easing(*easing)
                            .with_delay(*delay),
                    )
                };
            }
            Self::Playback { .. } => {}
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
            Self::Playback { elapsed, end } => {
                if let Some(end) = *end {
                    *elapsed = end;
                }
            }
        }
    }

    pub(crate) const fn value(&self) -> f32 {
        match self {
            Self::Spring(spring) => spring.value(),
            Self::Tween { current, .. } => *current,
            Self::Playback { elapsed, .. } => *elapsed,
        }
    }

    pub(crate) fn is_settled(&self) -> bool {
        match self {
            Self::Spring(spring) => spring.is_settled(),
            Self::Tween { tween, .. } => tween.is_none(),
            Self::Playback { elapsed, end } => end.is_some_and(|end| *elapsed >= end),
        }
    }

    pub(crate) const fn is_perpetual(&self) -> bool {
        matches!(self, Self::Playback { end: None, .. })
    }
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
