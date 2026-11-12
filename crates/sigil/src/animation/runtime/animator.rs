use crate::reactive::{Runtime, Signal};

use crate::animation::runtime::config::AnimationConfig;
use crate::animation::runtime::motion_prefs::MotionPrefs;
use crate::animation::runtime::state::{self, AnimatorId};

kittest_pub! {
    /// Animate a signal toward targets.
    #[derive(Debug)]
    pub struct Animator {
        id: Option<AnimatorId>,
        signal: Signal<f32>,
        config: AnimationConfig,
    }
}

impl Animator {
    #[cfg(feature = "kittest")]
    #[doc(hidden)]
    /// Creates an animator that drives `signal` with a spring preset.
    #[must_use]
    pub fn spring(signal: Signal<f32>, preset: crate::animation::SpringPreset) -> Self {
        Self::new(signal, AnimationConfig::Spring { preset })
    }

    #[cfg(feature = "kittest")]
    #[doc(hidden)]
    /// Creates an animator that drives `signal` with a tween of `ms` milliseconds.
    #[must_use]
    pub fn tween(signal: Signal<f32>, ms: u16, easing: crate::animation::Easing) -> Self {
        Self::new(
            signal,
            AnimationConfig::Tween {
                ms,
                easing,
                delay_ms: 0,
            },
        )
    }

    pub(crate) fn new(signal: Signal<f32>, config: AnimationConfig) -> Self {
        Self {
            id: (!MotionPrefs::reduced()).then(|| state::register_config(signal, config)),
            signal,
            config,
        }
    }

    pub(crate) fn set_config(&mut self, config: AnimationConfig) {
        if self.config != config {
            self.cancel();
            self.config = config;
        }
    }

    kittest_pub! {
        /// Animates toward `value`.
        pub fn target(&mut self, value: f32) {
            if !value.is_finite() {
                tracing::warn!(target: "sigil::animation", value, "ignored a non-finite animation target");
                return;
            }
            if MotionPrefs::reduced() {
                self.cancel();
                self.signal.try_set(value);
                Runtime::mark_frame_dirty();
                return;
            }
            let (signal, config) = (self.signal, self.config);
            self.id
                .get_or_insert_with(|| state::register_config(signal, config))
                .retarget(value);
        }
    }

    kittest_pub! {
        /// Jumps to the end of the animation.
        pub fn finish(&self) {
            if let Some(id) = self.id {
                id.finish();
            }
        }
    }

    kittest_pub! {
        /// Stops the animation where it is.
        pub fn cancel(&mut self) {
            if let Some(id) = self.id.take() {
                id.deregister();
            }
        }
    }

    #[cfg(feature = "kittest")]
    #[doc(hidden)]
    /// The current animated value, read without subscribing; 0.0 when the signal is gone.
    #[must_use]
    pub fn value(&self) -> f32 {
        self.id
            .and_then(AnimatorId::peek_value)
            .or_else(|| self.signal.try_peek())
            .unwrap_or_default()
    }

    kittest_pub! {
        /// Returns whether settled is set.
        #[must_use]
        pub fn settled(&self) -> bool {
            self.id.is_none_or(AnimatorId::is_settled)
        }
    }
}

impl Drop for Animator {
    fn drop(&mut self) {
        self.cancel();
    }
}
