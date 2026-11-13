use crate::animation::Motion;
use crate::reactive::frame::{FrameSource, SourceId, Sources, try_with_sources, with_sources};
use crate::reactive::{Runtime, Signal, batch};

use crate::animation::runtime::{AnimationConfig, MotionPrefs};

#[derive(Clone, Copy, Debug)]
pub(crate) struct AnimatorId(SourceId);

pub(crate) struct AnimatorSource {
    signal: Signal<f32>,
    active: Option<Signal<bool>>,
    motion: Motion,
}

pub(crate) fn register_config(
    signal: Signal<f32>,
    active: Option<Signal<bool>>,
    config: AnimationConfig,
) -> AnimatorId {
    let initial = signal.try_peek().unwrap_or_default();
    let motion = match config {
        AnimationConfig::Spring(spring) => Motion::spring(initial, spring),
        AnimationConfig::Tween {
            ms,
            easing,
            delay_ms,
        } => Motion::tween(initial, ms, easing, delay_ms),
        AnimationConfig::Playback { end, cycle } => Motion::playback(end, cycle),
    };
    register(signal, active, motion)
}

pub(crate) fn register(
    signal: Signal<f32>,
    active: Option<Signal<bool>>,
    motion: Motion,
) -> AnimatorId {
    assert!(
        signal.try_peek().is_none_or(f32::is_finite),
        "animation value must be finite"
    );
    let source = FrameSource::Animator(AnimatorSource {
        signal,
        active,
        motion,
    });
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
                if let Some(active) = entry.active {
                    active.try_set(!entry.motion.is_settled());
                }
                if !entry.signal.try_set(entry.motion.value()) {
                    sources.map.remove(self.0);
                }
                Runtime::mark_frame_dirty();
            });
        });
    }

    pub(crate) fn deregister(self) {
        let _ = try_with_sources(|sources| {
            sources.map.remove(self.0);
        });
    }

    pub(crate) fn retarget(self, value: f32) {
        batch(|| {
            with_sources(|sources| {
                if let Some(entry) = animator_mut(sources, self.0) {
                    entry.motion.retarget(value);
                    Runtime::mark_frame_dirty();
                    let settled = entry.motion.is_settled();
                    if settled {
                        entry.motion.finish();
                    }
                    if let Some(active) = entry.active {
                        active.try_set(!settled);
                    }
                    if settled && !entry.signal.try_set(entry.motion.value()) {
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

    #[cfg(test)]
    pub(crate) fn peek_value(self) -> Option<f32> {
        with_sources(|sources| animator_mut(sources, self.0).map(|entry| entry.motion.value()))
    }
}

#[cfg(host)]
pub(crate) fn finish_all() {
    batch(|| {
        retain_animators(|entry| {
            entry.motion.finish();
            if let Some(active) = entry.active {
                active.try_set(false);
            }
            entry.signal.try_set(entry.motion.value())
        });
        Runtime::mark_frame_dirty();
    });
}

kittest_pub! {
    /// Advances every running animation by `dt` seconds.
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
                if entry.motion.is_settled()
                    && let Some(active) = entry.active
                {
                    active.try_set(false);
                }
                entry.signal.try_set(entry.motion.value())
            });
        });
    }
}

kittest_pub! {
    /// Returns the number of animations that are not settled and not perpetual.
    #[must_use]
    pub fn active_count() -> usize {
        with_sources(|sources| {
            animators(sources)
                .filter(|entry| !entry.motion.is_settled() && !entry.motion.is_perpetual())
                .count()
        })
    }
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
