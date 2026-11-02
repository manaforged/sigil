use super::{Easing, Spring, SpringPreset, Tween};

pub(crate) enum Motion {
    Spring(Spring),
    Tween {
        current: f32,
        secs: f32,
        easing: Easing,
        tween: Option<Tween>,
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

    pub(crate) fn tick(&mut self, dt: f32) {
        match self {
            Self::Spring(spring) => spring.advance(dt),
            Self::Tween { current, tween, .. } => step_tween(current, tween, dt),
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
        }
    }

    pub(crate) const fn value(&self) -> f32 {
        match self {
            Self::Spring(spring) => spring.value(),
            Self::Tween { current, .. } => *current,
        }
    }

    pub(crate) fn is_settled(&self) -> bool {
        match self {
            Self::Spring(spring) => spring.is_settled(),
            Self::Tween { tween, .. } => tween.is_none(),
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
