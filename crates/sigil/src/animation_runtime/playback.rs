use std::sync::Arc;

use smallvec::SmallVec;

use crate::animation::{Keyframes, KeyframesId};
use crate::animation_runtime::motion::MotionPrefs;
use crate::animation_runtime::{AnimationConfig, Animator};
use crate::reactive::{Signal, signal};
use crate::style::{ColorAtom, ColorAtomKind, NumericAtomKind, Rgba8, StyleAtom, StyleEngine};

struct Playing {
    id: KeyframesId,
    keyframes: Arc<Keyframes>,
    animator: Animator,
}

pub(crate) struct Player {
    time: Signal<f32>,
    playing: Option<Playing>,
    colors: SmallVec<[(ColorAtom, Rgba8); 4]>,
}

pub(crate) fn animation(atoms: &[StyleAtom]) -> Option<KeyframesId> {
    atoms.iter().rev().find_map(|atom| match atom {
        StyleAtom::Animation(id) => Some(*id),
        _ => None,
    })
}

impl Player {
    pub(crate) fn new() -> Self {
        Self {
            time: signal(0.0_f32),
            playing: None,
            colors: SmallVec::new(),
        }
    }

    pub(crate) const fn time(&self) -> Signal<f32> {
        self.time
    }

    pub(crate) fn update(&mut self, atoms: &[StyleAtom], engine: &StyleEngine<'_>) {
        let next = animation(atoms);
        if next != self.playing.as_ref().map(|playing| playing.id) {
            self.playing = None;
            self.time.try_set(0.0);
            self.playing = next.and_then(|id| self.play(id));
        }
        self.colors.clear();
        if let Some(playing) = &self.playing {
            self.colors.extend(
                playing
                    .keyframes
                    .colors()
                    .map(|atom| (atom, engine.resolve_color(atom))),
            );
        }
    }

    fn play(&self, id: KeyframesId) -> Option<Playing> {
        let keyframes = id.get()?;
        let config = AnimationConfig::Playback {
            end: keyframes.end(),
            cycle: keyframes.cycle(),
        };
        Some(Playing {
            id,
            animator: Animator::new(self.time, config),
            keyframes,
        })
    }

    fn progress(&self) -> Option<(&Keyframes, f32)> {
        if MotionPrefs::reduced() {
            return None;
        }
        let playing = self
            .playing
            .as_ref()
            .filter(|playing| !playing.animator.settled())?;
        let progress = playing.keyframes.progress(self.time.try_peek()?)?;
        Some((&playing.keyframes, progress))
    }

    pub(crate) fn numeric(&self, kind: NumericAtomKind, underlying: f32) -> Option<f32> {
        let (keyframes, progress) = self.progress()?;
        keyframes.sample(progress, kind, underlying)
    }

    pub(crate) fn color(&self, kind: ColorAtomKind, underlying: Rgba8) -> Option<Rgba8> {
        let (keyframes, progress) = self.progress()?;
        let resolve = |atom: ColorAtom| {
            self.colors
                .iter()
                .find(|(key, _)| *key == atom)
                .map_or(underlying, |(_, color)| *color)
        };
        keyframes.sample_color(progress, kind, underlying, &resolve)
    }
}
