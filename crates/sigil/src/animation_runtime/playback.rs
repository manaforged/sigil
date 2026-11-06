use smallvec::SmallVec;

use crate::animation::Keyframes;
use crate::animation_runtime::motion::MotionPrefs;
use crate::animation_runtime::{AnimationConfig, Animator};
use crate::reactive::{Signal, signal};
use crate::style::{ColorAtom, ColorAtomKind, NumericAtomKind, Rgba8, StyleAtom, StyleEngine};

pub(crate) struct Player {
    time: Signal<f32>,
    playing: Option<(&'static Keyframes, Animator)>,
    colors: SmallVec<[(ColorAtom, Rgba8); 4]>,
}

pub(crate) fn animation(atoms: &[StyleAtom]) -> Option<&'static Keyframes> {
    atoms.iter().rev().find_map(|atom| match atom {
        StyleAtom::Animation(keyframes) => Some(*keyframes),
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
        let current = self.playing.as_ref().map(|(keyframes, _)| *keyframes);
        if next.map(std::ptr::from_ref) != current.map(std::ptr::from_ref) {
            self.playing = None;
            self.time.try_set(0.0);
            self.playing = next.map(|keyframes| {
                let config = AnimationConfig::Playback {
                    end: keyframes.end(),
                };
                (keyframes, Animator::new(self.time, config))
            });
        }
        self.colors.clear();
        if let Some((keyframes, _)) = self.playing {
            self.colors
                .extend(keyframes.colors().map(|atom| (atom, engine.resolve_color(atom))));
        }
    }

    fn progress(&self) -> Option<(&'static Keyframes, f32)> {
        if MotionPrefs::reduced() {
            return None;
        }
        let (keyframes, _) = self.playing.as_ref()?;
        let progress = keyframes.progress(self.time.try_peek()?)?;
        Some((keyframes, progress))
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
