//! Particle-pool simulation state shared between the spawn and tick
//! reactive effects, and surfaced through [`super::FctNode`] for
//! renderer / test inspection.

use std::cell::RefCell;

use sigil_reactive::Signal;
use sigil_style::Rgba8;

use super::spawn::{FctSpawn, Particle};
use super::style::FloaterStyle;

/// Maximum simultaneous particles tracked by one floating-text widget.
///
/// 1024 covers the "500 damage numbers per second" the host engine target with
/// headroom for ~2× simultaneous bursts.
pub const FCT_POOL_CAPACITY: usize = 1024;

/// Internal per-widget tuning accumulator, built by
/// [`super::FloatingTextBuilder`] and consumed once at
/// [`FctState::from_settings`].
#[derive(Clone, Copy, Debug)]
pub(super) struct FctSettings {
    pub(super) duration: f32,
    pub(super) drift: [f32; 2],
    pub(super) time: Option<Signal<f32>>,
    pub(super) capacity: usize,
    pub(super) anchor: Option<Signal<(f32, f32)>>,
    pub(super) style: Option<FloaterStyle>,
}

impl FctSettings {
    pub(super) const fn new() -> Self {
        Self {
            duration: 1.2,
            drift: [0.0, -32.0],
            time: None,
            capacity: FCT_POOL_CAPACITY,
            anchor: None,
            style: None,
        }
    }
}

/// Anchor + resolved-color options, bundled so [`FctState::build`]
/// stays under the 5-argument cap.
#[derive(Clone, Copy, Debug, Default)]
struct FctBuildOptions {
    anchor: Option<Signal<(f32, f32)>>,
    override_color: Option<Rgba8>,
}

/// Internal pool state, shared between spawn and tick effects and
/// surfaced through the node extension for renderer / test inspection.
#[derive(Debug)]
pub struct FctState {
    pool: RefCell<Vec<Particle>>,
    duration: f32,
    drift: [f32; 2],
    last_tick: RefCell<f32>,
    /// Screen-position signal a live particle tracks each tick instead
    /// of a fixed spawn-time origin — see
    /// [`super::FloatingTextBuilder::at`].
    anchor: Option<Signal<(f32, f32)>>,
    /// Resolved [`FloaterStyle`] color, when the widget was configured
    /// with [`super::FloatingTextBuilder::style`]. Wins over each
    /// spawn's own `color` field.
    override_color: Option<Rgba8>,
    /// Monotonic generation counter — bumps every spawn and every tick.
    /// Renderers can subscribe to skip work when nothing changed.
    pub generation: Signal<u32>,
}

impl FctState {
    /// Build a new pool with the supplied tuning.
    ///
    /// `duration` is clamped to `f32::EPSILON`; `capacity` is the fixed
    /// pool size and can be lower than [`FCT_POOL_CAPACITY`] for tests or
    /// specialized widgets.
    #[must_use]
    pub fn new(duration: f32, drift: [f32; 2], time: Signal<f32>, capacity: usize) -> Self {
        Self::build(duration, drift, time, capacity, FctBuildOptions::default())
    }

    /// Build a pool whose particles track `anchor`'s current position
    /// each tick instead of a fixed spawn-time origin — see
    /// [`super::FloatingTextBuilder::at`].
    #[must_use]
    pub fn new_anchored(
        duration: f32,
        drift: [f32; 2],
        time: Signal<f32>,
        capacity: usize,
        anchor: Signal<(f32, f32)>,
    ) -> Self {
        Self::build(
            duration,
            drift,
            time,
            capacity,
            FctBuildOptions {
                anchor: Some(anchor),
                override_color: None,
            },
        )
    }

    fn build(
        duration: f32,
        drift: [f32; 2],
        time: Signal<f32>,
        capacity: usize,
        opts: FctBuildOptions,
    ) -> Self {
        let pool = (0..capacity).map(|_| Particle::empty()).collect();
        Self {
            pool: RefCell::new(pool),
            duration: duration.max(f32::EPSILON),
            drift,
            last_tick: RefCell::new(time.peek()),
            anchor: opts.anchor,
            override_color: opts.override_color,
            generation: sigil_reactive::signal(0_u32),
        }
    }

    pub(super) fn from_settings(settings: FctSettings, time: Signal<f32>) -> Self {
        let opts = FctBuildOptions {
            anchor: settings.anchor,
            override_color: settings.style.map(FloaterStyle::color),
        };
        Self::build(
            settings.duration,
            settings.drift,
            time,
            settings.capacity,
            opts,
        )
    }

    /// Push a particle into the next free slot. Drops the spawn on the
    /// floor when the pool is saturated — preferable to growing the
    /// vector on the hot path. Returns `true` if the slot was found.
    pub fn spawn(&self, spawn: FctSpawn, now: f32) -> bool {
        if spawn.text.is_empty() {
            return false;
        }
        let mut pool = self.pool.borrow_mut();
        let Some(slot) = pool.iter_mut().find(|p| !p.alive) else {
            return false;
        };
        let color = self.override_color.unwrap_or(spawn.color);
        *slot = Particle {
            origin: spawn.at,
            drift: self.drift,
            position: spawn.at,
            spawn_time: now,
            remaining: self.duration,
            alpha: 1.0,
            color,
            text: spawn.text,
            alive: true,
        };
        true
    }

    /// Advance the simulation to `now` seconds. Linear over active
    /// particles. Dead slots are skipped via the `alive` flag.
    pub fn tick(&self, now: f32) {
        let dt = {
            let mut last = self.last_tick.borrow_mut();
            let dt = (now - *last).max(0.0);
            *last = now;
            dt
        };
        if dt == 0.0 {
            return;
        }
        let inv_dur = 1.0 / self.duration;
        let anchor_now = self.anchor.map(Signal::get);
        let mut pool = self.pool.borrow_mut();
        for slot in pool.iter_mut() {
            advance_slot(slot, dt, inv_dur, now, anchor_now);
        }
    }

    /// Read a flat snapshot of live particles. Allocates a fresh `Vec`;
    /// renderers that care about allocation may iterate directly via
    /// [`FctState::for_each_alive`].
    #[must_use]
    pub fn snapshot(&self) -> Vec<Particle> {
        self.pool
            .borrow()
            .iter()
            .filter(|p| p.alive)
            .cloned()
            .collect()
    }

    /// Iterate live particles without allocating.
    pub fn for_each_alive(&self, mut visit: impl FnMut(&Particle)) {
        for slot in self.pool.borrow().iter() {
            if slot.alive {
                visit(slot);
            }
        }
    }

    /// Count live particles (test helper).
    #[must_use]
    pub fn alive_count(&self) -> usize {
        self.pool.borrow().iter().filter(|p| p.alive).count()
    }
}

/// Advance one slot by `dt` seconds. When `anchor` is `Some`, position
/// is recomputed each tick from the anchor's current value plus the
/// elapsed-time drift offset (so a moving anchor drags the particle
/// along); otherwise position accumulates incrementally from the
/// spawn-time origin, matching the pre-anchor behavior.
fn advance_slot(slot: &mut Particle, dt: f32, inv_dur: f32, now: f32, anchor: Option<(f32, f32)>) {
    if !slot.alive {
        return;
    }
    slot.remaining -= dt;
    if slot.remaining <= 0.0 {
        *slot = Particle::empty();
        return;
    }
    if let Some((anchor_x, anchor_y)) = anchor {
        let elapsed = (now - slot.spawn_time).max(0.0);
        slot.position[0] = slot.drift[0].mul_add(elapsed, anchor_x);
        slot.position[1] = slot.drift[1].mul_add(elapsed, anchor_y);
    } else {
        slot.position[0] = slot.drift[0].mul_add(dt, slot.position[0]);
        slot.position[1] = slot.drift[1].mul_add(dt, slot.position[1]);
    }
    slot.alpha = (slot.remaining * inv_dur).clamp(0.0, 1.0);
}
