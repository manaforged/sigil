use super::AnimatorStatus;
use super::animator::{SpringMotion, TweenMotion};
use super::keyframes::KeyframeMotion;

pub(crate) enum Motion {
    Spring(SpringMotion),
    Tween(TweenMotion),
    Keyframes(KeyframeMotion),
}

impl Motion {
    pub(crate) fn tick(&mut self, dt: f32) -> AnimatorStatus {
        match self {
            Self::Spring(motion) => motion.tick(dt),
            Self::Tween(motion) => motion.tick(dt),
            Self::Keyframes(motion) => motion.tick(dt),
        }
    }

    pub(crate) fn retarget(&mut self, value: f32) {
        match self {
            Self::Spring(motion) => motion.retarget(value),
            Self::Tween(motion) => motion.retarget(value),
            Self::Keyframes(_) => unreachable!("only transition handles retarget"),
        }
    }

    pub(crate) fn finish(&mut self) {
        match self {
            Self::Spring(motion) => motion.finish(),
            Self::Tween(motion) => motion.finish(),
            Self::Keyframes(motion) => motion.finish(),
        }
    }

    pub(crate) const fn value(&self) -> f32 {
        match self {
            Self::Spring(motion) => motion.value(),
            Self::Tween(motion) => motion.value(),
            Self::Keyframes(motion) => motion.value(),
        }
    }

    pub(crate) fn is_settled(&self) -> bool {
        match self {
            Self::Spring(motion) => motion.is_settled(),
            Self::Tween(motion) => motion.is_settled(),
            Self::Keyframes(motion) => motion.is_settled(),
        }
    }
}
