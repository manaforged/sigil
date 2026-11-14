use std::sync::{Mutex, PoisonError};

use crate::animation::{Easing, Spring};
use crate::style::{ColorAtomKind, Family, NumericAtomKind, StyleAtom, TransitionId};

use crate::animation::runtime::config::AnimationConfig;

kittest_pub! {
    /// The ids of the built-in transitions.
    pub mod presets {
        use crate::style::TransitionId;

        pub(crate) const SMOOTH: TransitionId = TransitionId(0);
        pub(crate) const GENTLE: TransitionId = TransitionId(1);
        pub(crate) const SNAPPY: TransitionId = TransitionId(2);
        pub(crate) const BOUNCY: TransitionId = TransitionId(3);
        pub(crate) const STIFF: TransitionId = TransitionId(4);
        pub(crate) const FAST: TransitionId = TransitionId(16);
        kittest_pub! {
            /// The default transition: 200 ms, ease in and out.
            pub const DEFAULT: TransitionId = TransitionId(17);
        }
        pub(crate) const SLOW: TransitionId = TransitionId(18);
        pub(crate) const TRANSITION: TransitionId = TransitionId(19);
        pub(crate) const COLORS: TransitionId = TransitionId(20);
        pub(crate) const OPACITY: TransitionId = TransitionId(21);
        pub(crate) const TRANSFORM: TransitionId = TransitionId(22);
        kittest_pub! {
            /// The transition that animates every property.
            pub const ALL: TransitionId = TransitionId(23);
        }
    }
}

pub(crate) const TWEEN_MS: u16 = 150;

const SPRING_BASE: u16 = 0x8000;

const LAYOUT: u16 = 0x4000;

pub(crate) const fn with_layout(id: TransitionId) -> TransitionId {
    TransitionId(id.0 | LAYOUT)
}

const fn timed(id: TransitionId) -> TransitionId {
    TransitionId(id.0 & !LAYOUT)
}

const SPRING_PRESETS: [TransitionId; 5] = [
    presets::SMOOTH,
    presets::GENTLE,
    presets::SNAPPY,
    presets::BOUNCY,
    presets::STIFF,
];

static SPRINGS: Mutex<Vec<Spring>> = Mutex::new(Vec::new());

pub(crate) fn intern(spring: Spring) -> TransitionId {
    let wanted = Some(AnimationConfig::Spring(spring));
    if let Some(id) = SPRING_PRESETS.into_iter().find(|id| config(*id) == wanted) {
        return id;
    }
    let mut springs = SPRINGS.lock().unwrap_or_else(PoisonError::into_inner);
    let index = springs.iter().position(|held| *held == spring).unwrap_or_else(|| {
        springs.push(spring);
        springs.len() - 1
    });
    let id = u16::try_from(index)
        .ok()
        .filter(|index| *index < LAYOUT)
        .and_then(|index| SPRING_BASE.checked_add(index));
    id.map_or_else(
        || {
            springs.truncate(index);
            tracing::warn!(target: "sigil::animation", "too many distinct transition springs; using the smooth spring");
            presets::SMOOTH
        },
        TransitionId,
    )
}

fn interned(id: TransitionId) -> Option<AnimationConfig> {
    let index = usize::from(timed(id).0.checked_sub(SPRING_BASE)?);
    let springs = SPRINGS.lock().unwrap_or_else(PoisonError::into_inner);
    springs.get(index).copied().map(AnimationConfig::Spring)
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
        match timed(id) {
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
    let spring = match timed(id) {
        presets::SMOOTH => Spring::SMOOTH,
        presets::GENTLE => Spring::GENTLE,
        presets::SNAPPY => Spring::SNAPPY,
        presets::BOUNCY => Spring::BOUNCY,
        presets::STIFF => Spring::STIFF,
        presets::FAST => return Some(cubic_in_out(120)),
        presets::DEFAULT => return Some(cubic_in_out(200)),
        presets::SLOW => return Some(cubic_in_out(320)),
        presets::TRANSITION
        | presets::COLORS
        | presets::OPACITY
        | presets::TRANSFORM
        | presets::ALL => {
            return Some(AnimationConfig::Tween {
                ms: TWEEN_MS,
                easing: Easing::IN_OUT,
                delay_ms: 0,
            });
        }
        _ => return None,
    };
    Some(AnimationConfig::Spring(spring))
}

pub(crate) const fn timing(id: TransitionId) -> (u16, Easing, u16) {
    match config(id) {
        Some(AnimationConfig::Tween {
            ms,
            easing,
            delay_ms,
        }) => (ms, easing, delay_ms),
        _ => timing(presets::TRANSITION),
    }
}

const fn cubic_in_out(ms: u16) -> AnimationConfig {
    AnimationConfig::Tween {
        ms,
        easing: Easing::CubicInOut,
        delay_ms: 0,
    }
}

fn transition_id(atoms: impl IntoIterator<Item = StyleAtom>) -> Option<TransitionId> {
    atoms
        .into_iter()
        .filter_map(|atom| match atom {
            StyleAtom::Transition(id) => Some(id),
            _ => None,
        })
        .last()
}

pub(crate) fn glides(atoms: impl IntoIterator<Item = StyleAtom>) -> bool {
    transition_id(atoms).is_some_and(|id| id.0 & LAYOUT != 0)
}

pub(crate) fn glide_config(atoms: &[StyleAtom]) -> Option<AnimationConfig> {
    glides(atoms.iter().copied())
        .then(|| transition_config(atoms))
        .flatten()
        .map(|(config, _)| config)
}

pub(crate) fn transition_config(atoms: &[StyleAtom]) -> Option<(AnimationConfig, Transitioned)> {
    let id = transition_id(atoms.iter().copied())?;
    let mut config = config(id).or_else(|| interned(id))?;
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
