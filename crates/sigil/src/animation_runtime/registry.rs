use crate::animation::{Easing, SpringPreset};
use crate::style::{ColorAtomKind, Family, NumericAtomKind, StyleAtom, TransitionId};

use crate::animation_runtime::config::AnimationConfig;

pub mod presets {
    use crate::style::TransitionId;

    pub const SMOOTH: TransitionId = TransitionId(0);
    pub const GENTLE: TransitionId = TransitionId(1);
    pub const SNAPPY: TransitionId = TransitionId(2);
    pub const BOUNCY: TransitionId = TransitionId(3);
    pub const STIFF: TransitionId = TransitionId(4);
    pub const FAST: TransitionId = TransitionId(16);
    pub const DEFAULT: TransitionId = TransitionId(17);
    pub const SLOW: TransitionId = TransitionId(18);
    pub const TRANSITION: TransitionId = TransitionId(19);
    pub const COLORS: TransitionId = TransitionId(20);
    pub const OPACITY: TransitionId = TransitionId(21);
    pub const TRANSFORM: TransitionId = TransitionId(22);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Transitioned {
    All,
    Paint,
    Colors,
    Opacity,
    Transform,
}

impl Transitioned {
    pub(crate) const fn of(id: TransitionId) -> Self {
        match id {
            presets::TRANSITION => Self::Paint,
            presets::COLORS => Self::Colors,
            presets::OPACITY => Self::Opacity,
            presets::TRANSFORM => Self::Transform,
            _ => Self::All,
        }
    }

    pub(crate) const fn numeric(self, kind: NumericAtomKind) -> bool {
        let transform = matches!(
            kind,
            NumericAtomKind::TranslateX
                | NumericAtomKind::TranslateY
                | NumericAtomKind::TranslateXFraction
                | NumericAtomKind::TranslateYFraction
                | NumericAtomKind::ScaleX
                | NumericAtomKind::ScaleY
                | NumericAtomKind::Rotation
                | NumericAtomKind::RotationX
                | NumericAtomKind::RotationY
                | NumericAtomKind::RotationZ
                | NumericAtomKind::TranslateZ
        );
        let opacity = matches!(kind, NumericAtomKind::Opacity);
        let filter = matches!(kind.family(), Family::Filter);
        match self {
            Self::All => true,
            Self::Paint => transform || opacity || filter,
            Self::Colors => false,
            Self::Opacity => opacity,
            Self::Transform => transform,
        }
    }

    pub(crate) const fn color(self, kind: ColorAtomKind) -> bool {
        match self {
            Self::All => true,
            Self::Paint => !matches!(kind, ColorAtomKind::TextShadow),
            Self::Colors => matches!(kind.family(), Family::Plain | Family::Gradient),
            Self::Opacity | Self::Transform => false,
        }
    }
}

#[must_use]
pub(crate) const fn config(id: TransitionId) -> Option<AnimationConfig> {
    let preset = match id {
        presets::SMOOTH => SpringPreset::Smooth,
        presets::GENTLE => SpringPreset::Gentle,
        presets::SNAPPY => SpringPreset::Snappy,
        presets::BOUNCY => SpringPreset::Bouncy,
        presets::STIFF => SpringPreset::Stiff,
        presets::FAST => return Some(cubic_in_out(120)),
        presets::DEFAULT => return Some(cubic_in_out(200)),
        presets::SLOW => return Some(cubic_in_out(320)),
        presets::TRANSITION | presets::COLORS | presets::OPACITY | presets::TRANSFORM => {
            return Some(AnimationConfig::Tween {
                ms: 150,
                easing: Easing::IN_OUT,
                delay_ms: 0,
            });
        }
        _ => return None,
    };
    Some(AnimationConfig::Spring { preset })
}

const fn cubic_in_out(ms: u16) -> AnimationConfig {
    AnimationConfig::Tween {
        ms,
        easing: Easing::CubicInOut,
        delay_ms: 0,
    }
}

pub(crate) fn transition_config(atoms: &[StyleAtom]) -> Option<(AnimationConfig, Transitioned)> {
    let id = atoms.iter().rev().find_map(|atom| match atom {
        StyleAtom::Transition(id) => Some(*id),
        _ => None,
    })?;
    let mut config = config(id)?;
    if let AnimationConfig::Tween {
        ms,
        easing,
        delay_ms,
    } = &mut config
    {
        for atom in atoms {
            match atom {
                StyleAtom::TransitionDuration(duration) => *ms = duration.0,
                StyleAtom::TransitionEase(ease) => *easing = *ease,
                StyleAtom::TransitionDelay(delay) => *delay_ms = delay.0,
                _ => {}
            }
        }
    }
    Some((config, Transitioned::of(id)))
}
