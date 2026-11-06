use crate::animation::Motion;
use crate::reactive::frame::{FrameSource, SourceId, Sources, with_sources};
use crate::reactive::{Runtime, Signal, batch};

use crate::animation_runtime::{AnimationConfig, MotionPrefs};

#[derive(Clone, Copy)]
pub(crate) struct AnimatorId(SourceId);

pub(crate) struct AnimatorSource {
    signal: Signal<f32>,
    motion: Motion,
}

pub(crate) fn register_config(signal: Signal<f32>, config: AnimationConfig) -> AnimatorId {
    let initial = signal.try_peek().unwrap_or_default();
    let motion = match config {
        AnimationConfig::Spring { preset } => Motion::spring(initial, preset),
        AnimationConfig::Tween { ms, easing, delay_ms } => {
            Motion::tween(initial, ms, easing, delay_ms)
        }
        AnimationConfig::Playback { end, cycle } => Motion::playback(end, cycle),
    };
    register(signal, motion)
}

pub(crate) fn register(signal: Signal<f32>, motion: Motion) -> AnimatorId {
    assert!(
        signal.try_peek().is_none_or(f32::is_finite),
        "animation value must be finite"
    );
    let source = FrameSource::Animator(AnimatorSource { signal, motion });
    AnimatorId(with_sources(|sources| sources.map.insert(source)))
}

fn animator_mut(sources: &mut Sources, id: SourceId) -> Option<&mut AnimatorSource> {
    match sources.map.get_mut(id) {
        Some(FrameSource::Animator(entry)) => Some(entry),
        _ => None,
    }
}

fn animators(sources: &Sources) -> impl Iterator<Item = &AnimatorSource> {
    sources.map.values().filter_map(|source| match source {
        FrameSource::Animator(entry) => Some(entry),
        _ => None,
    })
}

fn retain_animators(mut keep: impl FnMut(&mut AnimatorSource) -> bool) {
    with_sources(|sources| {
        sources.map.retain(|_, source| match source {
            FrameSource::Animator(entry) => keep(entry),
            _ => true,
        });
    });
}

impl AnimatorId {
    pub(crate) fn finish(self) {
        self.update(|entry| entry.motion.finish());
    }

    fn update(self, change: impl FnOnce(&mut AnimatorSource)) {
        batch(|| {
            with_sources(|sources| {
                let Some(entry) = animator_mut(sources, self.0) else {
                    return;
                };
                change(entry);
                if MotionPrefs::reduced() && !entry.motion.is_settled() {
                    entry.motion.finish();
                }
                if !entry.signal.try_set(entry.motion.value()) {
                    sources.map.remove(self.0);
                }
                Runtime::mark_frame_dirty();
            });
        });
    }

    pub(crate) fn deregister(self) {
        with_sources(|sources| {
            sources.map.remove(self.0);
        });
    }

    pub(crate) fn retarget(self, value: f32) {
        batch(|| {
            with_sources(|sources| {
                if let Some(entry) = animator_mut(sources, self.0) {
                    entry.motion.retarget(value);
                    Runtime::mark_frame_dirty();
                    if entry.motion.is_settled() && !entry.signal.try_set(entry.motion.value()) {
                        sources.map.remove(self.0);
                    }
                }
            });
        });
    }

    pub(crate) fn is_settled(self) -> bool {
        with_sources(|sources| {
            animator_mut(sources, self.0).is_none_or(|entry| entry.motion.is_settled())
        })
    }

    #[cfg(any(test, feature = "kittest"))]
    pub(crate) fn peek_value(self) -> Option<f32> {
        with_sources(|sources| animator_mut(sources, self.0).map(|entry| entry.motion.value()))
    }
}

#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
pub(crate) fn finish_all() {
    batch(|| {
        retain_animators(|entry| {
            entry.motion.finish();
            entry.signal.try_set(entry.motion.value())
        });
        Runtime::mark_frame_dirty();
    });
}

pub fn tick(dt: f32) {
    if !dt.is_finite() || dt <= 0.0 {
        return;
    }
    batch(|| {
        retain_animators(|entry| {
            if entry.motion.is_settled() {
                return true;
            }
            entry.motion.tick(dt);
            Runtime::mark_frame_dirty();
            entry.signal.try_set(entry.motion.value())
        });
    });
}

#[must_use]
pub fn active_count() -> usize {
    with_sources(|sources| {
        animators(sources)
            .filter(|entry| !entry.motion.is_settled() && !entry.motion.is_perpetual())
            .count()
    })
}

pub(crate) fn perpetual_count() -> usize {
    with_sources(|sources| {
        animators(sources)
            .filter(|entry| entry.motion.is_perpetual())
            .count()
    })
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
