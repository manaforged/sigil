//! Reactive a11y observer — W5.2 / DR-050 §G10-phase-2.
//!
//! [`A11yObserver`] is a long-lived reactive owner that holds one
//! tracking effect per signal-backed accessibility property
//! (`checked`, `value_now`, `expanded`, `text_value`, …). When any of
//! those signals changes, its effect re-runs and flips the runtime's
//! [`a11y_dirty`](crate::runtime::RuntimeState::a11y_dirty) flag.
//!
//! At end-of-frame [`crate::runtime_drain::drain_a11y_updates`] sees
//! the flag, clears it, and invokes the publisher closure installed
//! via [`crate::Runtime::install_a11y_publisher`]; the publisher is
//! responsible for snapshotting the kernel `Cx` into a `TreeUpdate`
//! and forwarding it to the bridge.
//!
//! Disposing the [`A11yObserver`] (via `Drop` or
//! [`A11yObserver::dispose`]) cascades through the owner tree and
//! tears down every registered effect — no further dirty marks fire
//! from the released subscriptions.

use crate::effect::effect;
use crate::owner::Owner;
use crate::runtime::with_rt;
use crate::signal::Signal;

/// Long-lived reactive owner that tracks a UI subtree's a11y signals.
///
/// Construct one [`A11yObserver`] per `Cx` (typically: one per window).
/// Call [`A11yObserver::observe_bool`], [`observe_f32`], and
/// [`observe_string`](Self::observe_string) for each signal-backed
/// `A11yProps` field of every node the shell has added. The internal
/// effects re-run on every change and mark the runtime tree dirty.
///
/// `!Send + !Sync` — the reactive runtime is thread-local.
///
/// [`observe_f32`]: Self::observe_f32
#[must_use = "A11yObserver disposes its tracking effects on drop; bind it for the lifetime of the watched UI"]
#[derive(Debug)]
pub struct A11yObserver {
    owner: Owner,
}

impl A11yObserver {
    /// Create a new observer parented to the current reactive owner.
    ///
    /// Tracking effects registered through this observer survive until
    /// either the observer is dropped or its parent owner is disposed,
    /// whichever comes first.
    ///
    /// # Panics
    ///
    /// Panics if called outside any reactive scope. Use
    /// [`crate::Runtime::scope`] or [`Owner::with`] to establish one.
    pub fn new() -> Self {
        let parent = Owner::current();
        let owner = Owner::new("sigil-reactive::a11y-observer", parent);
        Self { owner }
    }

    /// Subscribe to a `Signal<bool>` — covers `checked` and `expanded`.
    ///
    /// The body reads the signal (registering the dependency) and
    /// flips the runtime's `a11y_dirty` flag. The body is a closure
    /// owned by this observer's `Owner`; disposal cascades.
    pub fn observe_bool(&self, signal: Signal<bool>) {
        self.owner.with(|| {
            effect(move || {
                let _ = signal.get();
                mark_dirty();
            });
        });
    }

    /// Subscribe to a `Signal<f32>` — covers `value_now` (slider /
    /// progress / spinbutton).
    pub fn observe_f32(&self, signal: Signal<f32>) {
        self.owner.with(|| {
            effect(move || {
                let _ = signal.get();
                mark_dirty();
            });
        });
    }

    /// Subscribe to a `Signal<String>` — covers `text_value` and any
    /// derived accessible name / description.
    pub fn observe_string(&self, signal: Signal<String>) {
        self.owner.with(|| {
            effect(move || {
                let _ = signal.get();
                mark_dirty();
            });
        });
    }

    /// Dispose this observer's owner, eagerly tearing down every
    /// tracking effect. Idempotent on drop — calling `dispose` and
    /// then letting the value drop is safe; the second cascade is a
    /// no-op against an already-disposed owner.
    pub fn dispose(self) {
        self.owner.dispose();
    }
}

impl Default for A11yObserver {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for A11yObserver {
    fn drop(&mut self) {
        // Owner::dispose is idempotent against a disposed owner.
        self.owner.dispose();
    }
}

/// Flip the runtime's `a11y_dirty` flag. Cheap (one `with_rt` borrow,
/// one bool assign). Called from every tracking effect body.
pub(crate) fn mark_dirty() {
    with_rt(|rt| rt.a11y_dirty = true);
}
