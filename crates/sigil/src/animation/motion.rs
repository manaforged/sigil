use super::{Easing, Keyframes, Spring, SpringPreset, Tween};

pub(crate) enum Motion {
    Spring(Spring),
    Tween {
        current: f32,
        secs: f32,
        easing: Easing,
        tween: Option<Tween>,
    },
    Keyframes {
        frames: Keyframes,
        elapsed: f64,
        initial: f32,
        current: f32,
        finished: bool,
    },
}

impl Motion {
    pub(crate) fn spring(initial: f32, preset: SpringPreset) -> Self {
        Self::Spring(Spring::from_preset(initial, preset))
    }

    pub(crate) const fn tween(initial: f32, ms: u16, easing: Easing) -> Self {
        Self::Tween {
            current: initial,
            secs: ms as f32 / 1000.0,
            easing,
            tween: None,
        }
    }

    pub(crate) fn keyframes(frames: Keyframes, initial: f32) -> Self {
        let mut motion = Self::Keyframes {
            frames,
            elapsed: 0.0,
            initial,
            current: initial,
            finished: false,
        };
        motion.seek(0.0);
        motion
    }

    pub(crate) fn tick(&mut self, dt: f32) {
        let seconds = match self {
            Self::Spring(spring) => return spring.advance(dt),
            Self::Tween { current, tween, .. } => return step_tween(current, tween, dt),
            Self::Keyframes { elapsed, .. } => *elapsed + f64::from(dt),
        };
        self.seek(seconds);
    }

    pub(crate) fn seek(&mut self, seconds: f64) {
        let Self::Keyframes {
            frames,
            elapsed,
            initial,
            current,
            finished,
        } = self
        else {
            return;
        };
        *elapsed = seconds;
        if let Some(value) = frames.at(seconds, *initial) {
            *current = value;
            *finished = false;
        } else {
            *current = frames.end_value(*initial);
            *finished = true;
        }
    }

    pub(crate) fn retarget(&mut self, value: f32) {
        match self {
            Self::Spring(spring) => spring.set_target(value),
            Self::Tween {
                current,
                secs,
                easing,
                tween,
            } => {
                *tween = if *secs <= 0.0 {
                    *current = value;
                    None
                } else if (*current - value).abs() < f32::EPSILON {
                    None
                } else {
                    Some(Tween::new(*current, value, *secs).with_easing(*easing))
                };
            }
            Self::Keyframes { .. } => unreachable!("only transition handles retarget"),
        }
    }

    pub(crate) fn finish(&mut self) {
        match self {
            Self::Spring(spring) => spring.reset(spring.target()),
            Self::Tween { current, tween, .. } => {
                if let Some(active) = tween.take() {
                    *current = active.to();
                }
            }
            Self::Keyframes {
                frames,
                initial,
                current,
                finished,
                ..
            } => {
                *current = frames.end_value(*initial);
                *finished = true;
            }
        }
    }

    pub(crate) const fn cancel(&mut self) {
        if let Self::Keyframes {
            initial,
            current,
            finished,
            ..
        } = self
        {
            *current = *initial;
            *finished = true;
        }
    }

    pub(crate) const fn value(&self) -> f32 {
        match self {
            Self::Spring(spring) => spring.value(),
            Self::Tween { current, .. } | Self::Keyframes { current, .. } => *current,
        }
    }

    pub(crate) fn is_settled(&self) -> bool {
        match self {
            Self::Spring(spring) => spring.is_settled(),
            Self::Tween { tween, .. } => tween.is_none(),
            Self::Keyframes { finished, .. } => *finished,
        }
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
