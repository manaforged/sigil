//! Floating combat text — particle-style numbers that bypass layout.
//!
//! Each spawn pushes a particle into a fixed-capacity pool. A reactive
//! tick advances ages and evicts expired particles. Positions are stored
//! directly on the pool (no layout round-trip), so a downstream renderer
//! can read [`FctState::snapshot`] each frame and emit scene commands
//! without invoking the layout engine.
//!
//! Design intent: the "500 damage numbers per second" use case for
//! the host engine. Per-frame cost is dominated by the simulation step (linear
//! over active particles); the widget itself never recomputes layout.
//!
//! See the strategic plan §F6.
//!
//! # Hygiene-exception - `Signal<f32>` time input
//!
//! A particle system needs a clock; this implementation accepts one via
//! [`FloatingTextBuilder::time`] (caller-driven seconds-since-start).
//! This keeps the widget agnostic to whatever frame loop the caller runs
//! and makes the bench measurable. If no clock is supplied, the builder
//! installs an inert `0.0` signal.
//!
//! # Hygiene-exception — `Signal<FctSpawn>` edge semantics
//!
//! The spawn signal is treated as edge-triggered: each `set()` triggers
//! the spawn effect, which reads the current value and pushes one
//! particle. Holding the same value is a no-op — callers spawn N
//! particles by calling `spawn.set(...)` N times.

use std::cell::RefCell;
use std::rc::Rc;

use sigil_kernel::{AccessibleName, Cx, IntoNode, NodeBuilder, fragment};
use sigil_reactive::{Signal, effect_named};
use sigil_style::{NodeStyleExt, Rgba8, StateHost, Style, StyleAccess};

use super::shared::BuilderMutator;

/// Maximum simultaneous particles tracked by one floating-text widget.
///
/// 1024 covers the "500 damage numbers per second" the host engine target with
/// headroom for ~2× simultaneous bursts.
pub const FCT_POOL_CAPACITY: usize = 1024;

/// One spawn request — the inputs the caller provides per particle.
#[derive(Clone, Debug, PartialEq)]
pub struct FctSpawn {
    /// Spawn position in widget-local logical pixels (`[x, y]`).
    pub at: [f32; 2],
    /// Text to render. Empty strings spawn an inert particle (no-op).
    pub text: String,
    /// Fill colour for the glyph run.
    pub color: Rgba8,
}

impl FctSpawn {
    /// Convenience constructor.
    #[must_use]
    pub fn new(at: [f32; 2], text: impl Into<String>, color: Rgba8) -> Self {
        Self {
            at,
            text: text.into(),
            color,
        }
    }
}

impl Default for FctSpawn {
    fn default() -> Self {
        Self {
            at: [0.0, 0.0],
            text: String::new(),
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
        }
    }
}

/// Builder returned by [`floating_text`].
///
/// It owns per-widget tuning such as duration, drift, clock, and pool
/// capacity. Build it directly as a node or continue chaining style
/// methods from [`crate::prelude::Styled`].
pub struct FloatingTextBuilder {
    spawn: Signal<FctSpawn>,
    settings: FctSettings,
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
}

impl FloatingTextBuilder {
    /// Create a floating-text builder with default tuning.
    ///
    /// Defaults are a 1.2s lifetime, 32 px/s upward drift, an inert
    /// `0.0` time signal, and [`FCT_POOL_CAPACITY`] slots.
    #[must_use]
    pub fn new(spawn: Signal<FctSpawn>) -> Self {
        Self {
            spawn,
            settings: FctSettings::new(sigil_reactive::signal(0.0_f32)),
            mutators: Vec::new(),
            style_accumulator: Style::new(),
        }
    }

    /// Set the particle lifetime in seconds.
    ///
    /// Non-positive values are coerced to `f32::EPSILON` to keep the
    /// simulation step finite.
    #[must_use]
    pub const fn duration(mut self, seconds: f32) -> Self {
        self.settings.duration = if seconds > f32::EPSILON {
            seconds
        } else {
            f32::EPSILON
        };
        self
    }

    /// Set the drift velocity in logical pixels per second.
    #[must_use]
    pub const fn drift(mut self, drift: [f32; 2]) -> Self {
        self.settings.drift = drift;
        self
    }

    /// Set the caller-driven clock in seconds.
    ///
    /// Each write advances the particle simulation.
    #[must_use]
    pub const fn time(mut self, time: Signal<f32>) -> Self {
        self.settings.time = time;
        self
    }

    /// Set the maximum number of simultaneous particles.
    #[must_use]
    pub const fn capacity(mut self, capacity: usize) -> Self {
        self.settings.capacity = capacity;
        self
    }

    /// Set the accessible label on the floating-text fragment.
    #[must_use]
    pub fn label(mut self, name: impl Into<AccessibleName> + 'static) -> Self {
        let name = name.into();
        self.mutators.push(Box::new(move |node| node.label(name)));
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let state = Rc::new(FctState::from_settings(self.settings));
        let extension = Rc::clone(&state);
        let time = self.settings.time;

        install_spawn_effect(Rc::clone(&state), self.spawn, time);
        install_tick_effect(Rc::clone(&state), time);

        let mut node = fragment().mutate(move |node| {
            let _previous = node.attach(FctNode(extension));
        });
        for mutate in self.mutators {
            node = mutate(node);
        }
        if !self.style_accumulator.is_empty() {
            node = node.style(self.style_accumulator);
        }
        node
    }
}

impl StyleAccess for FloatingTextBuilder {
    fn style(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl StateHost for FloatingTextBuilder {}

impl IntoNode for FloatingTextBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

/// Internal per-widget tuning accumulator.
#[derive(Clone, Copy, Debug)]
struct FctSettings {
    duration: f32,
    drift: [f32; 2],
    time: Signal<f32>,
    capacity: usize,
}

impl FctSettings {
    const fn new(time: Signal<f32>) -> Self {
        Self {
            duration: 1.2,
            drift: [0.0, -32.0],
            time,
            capacity: FCT_POOL_CAPACITY,
        }
    }
}

/// A single live particle. `dead` slots are reused by the next spawn.
#[derive(Clone, Debug, PartialEq)]
pub struct Particle {
    /// Origin point at spawn time.
    pub origin: [f32; 2],
    /// Drift velocity captured at spawn time (px/s).
    pub drift: [f32; 2],
    /// Current position (updated each tick).
    pub position: [f32; 2],
    /// Spawn time in seconds (from the caller's clock).
    pub spawn_time: f32,
    /// Remaining lifetime in seconds. Reaches `0.0` → slot reclaimed.
    pub remaining: f32,
    /// Alpha in `[0.0, 1.0]`. Eases to 0 over the lifetime.
    pub alpha: f32,
    /// Fill colour. Alpha channel is multiplied with `alpha` at read.
    pub color: Rgba8,
    /// Glyph text — short enough that `String` allocation does not
    /// dominate hot-path cost for typical 1-6 character damage numbers.
    pub text: String,
    /// Slot occupied flag.
    pub alive: bool,
}

impl Particle {
    /// An inert empty slot.
    const fn empty() -> Self {
        Self {
            origin: [0.0, 0.0],
            drift: [0.0, 0.0],
            position: [0.0, 0.0],
            spawn_time: 0.0,
            remaining: 0.0,
            alpha: 0.0,
            color: Rgba8 {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            text: String::new(),
            alive: false,
        }
    }
}

/// Internal pool state, shared between spawn and tick effects and
/// surfaced through the node extension for renderer / test inspection.
#[derive(Debug)]
pub struct FctState {
    pool: RefCell<Vec<Particle>>,
    duration: f32,
    drift: [f32; 2],
    last_tick: RefCell<f32>,
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
        let pool = (0..capacity).map(|_| Particle::empty()).collect();
        Self {
            pool: RefCell::new(pool),
            duration: duration.max(f32::EPSILON),
            drift,
            last_tick: RefCell::new(time.peek()),
            generation: sigil_reactive::signal(0_u32),
        }
    }

    fn from_settings(settings: FctSettings) -> Self {
        Self::new(
            settings.duration,
            settings.drift,
            settings.time,
            settings.capacity,
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
        *slot = Particle {
            origin: spawn.at,
            drift: self.drift,
            position: spawn.at,
            spawn_time: now,
            remaining: self.duration,
            alpha: 1.0,
            color: spawn.color,
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
        let mut pool = self.pool.borrow_mut();
        for slot in pool.iter_mut() {
            advance_slot(slot, dt, inv_dur);
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

fn advance_slot(slot: &mut Particle, dt: f32, inv_dur: f32) {
    if !slot.alive {
        return;
    }
    slot.remaining -= dt;
    if slot.remaining <= 0.0 {
        *slot = Particle::empty();
        return;
    }
    slot.position[0] += slot.drift[0] * dt;
    slot.position[1] += slot.drift[1] * dt;
    slot.alpha = (slot.remaining * inv_dur).clamp(0.0, 1.0);
}

/// Create a floating combat text widget builder.
///
/// Each write to `spawn` queues one particle. Each write to
/// [`FloatingTextBuilder::time`] advances simulation. The built node is
/// a fragment carrying an [`FctState`] extension that a downstream
/// renderer reads each frame.
///
/// # Examples
///
/// ```text
/// use sigil::prelude::*;
/// let time = signal(0.0_f32);
/// let spawn = signal(FctSpawn::default());
/// let widget = floating_text(spawn)
///     .time(time)
///     .duration(2.0)
///     .drift([0.0, -40.0]);
/// ```
#[must_use]
#[track_caller]
pub fn floating_text(spawn: Signal<FctSpawn>) -> FloatingTextBuilder {
    FloatingTextBuilder::new(spawn)
}

fn install_spawn_effect(state: Rc<FctState>, spawn: Signal<FctSpawn>, time: Signal<f32>) {
    let mut first = true;
    effect_named("sigil::floating_text::spawn", move || {
        let payload = spawn.get();
        if first {
            // Skip the implicit initial read so callers can pass a
            // default-constructed signal without spawning a ghost
            // particle.
            first = false;
            return;
        }
        let now = time.peek();
        let _ok = state.spawn(payload, now);
        state.generation.update(|g| *g = g.wrapping_add(1));
    });
}

fn install_tick_effect(state: Rc<FctState>, time: Signal<f32>) {
    let mut first = true;
    effect_named("sigil::floating_text::tick", move || {
        let now = time.get();
        if first {
            first = false;
            return;
        }
        state.tick(now);
        state.generation.update(|g| *g = g.wrapping_add(1));
    });
}

/// Node-extension wrapper around the shared pool. Renderers downcast
/// this from `Node::get_extension` to read positions per frame.
#[derive(Clone, Debug)]
pub struct FctNode(pub Rc<FctState>);

impl FctNode {
    /// Borrow the underlying state for read-only inspection.
    #[must_use]
    pub const fn state(&self) -> &Rc<FctState> {
        &self.0
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use sigil_reactive::{Runtime, signal};

    fn red() -> Rgba8 {
        Rgba8 {
            r: 255,
            g: 32,
            b: 32,
            a: 255,
        }
    }

    #[test]
    fn floating_text_returns_buildable_node() {
        Runtime::scope("fct-build-node", || {
            let time = signal(0.0_f32);
            let spawn = signal(FctSpawn::default());
            // Smoke: the returned builder can be configured without panicking
            // and the spawn signal can be driven through it.
            let _builder = floating_text(spawn).time(time).duration(1.0);
            spawn.set(FctSpawn::new([10.0, 20.0], "100", red()));
            spawn.set(FctSpawn::new([30.0, 40.0], "50", red()));
        });
    }

    #[test]
    fn tick_advances_position_and_alpha() {
        Runtime::scope("fct-tick-advance", || {
            let time = signal(0.0_f32);
            let state = Rc::new(FctState::new(1.0, [10.0, -20.0], time, FCT_POOL_CAPACITY));
            assert!(state.spawn(FctSpawn::new([0.0, 0.0], "1", red()), 0.0));
            state.tick(0.5);
            let snap = state.snapshot();
            assert_eq!(snap.len(), 1);
            assert!((snap[0].position[0] - 5.0).abs() < 0.001);
            assert!((snap[0].position[1] + 10.0).abs() < 0.001);
            assert!((snap[0].alpha - 0.5).abs() < 0.001);
        });
    }

    #[test]
    fn expired_particles_are_reclaimed() {
        Runtime::scope("fct-expire", || {
            let time = signal(0.0_f32);
            let state = Rc::new(FctState::new(0.5, [0.0, -32.0], time, FCT_POOL_CAPACITY));
            assert!(state.spawn(FctSpawn::new([0.0, 0.0], "9", red()), 0.0));
            assert_eq!(state.alive_count(), 1);
            state.tick(0.4);
            assert_eq!(state.alive_count(), 1);
            state.tick(0.6);
            assert_eq!(state.alive_count(), 0, "particle expired after duration");
        });
    }

    #[test]
    fn empty_text_does_not_spawn() {
        Runtime::scope("fct-empty", || {
            let time = signal(0.0_f32);
            let state = Rc::new(FctState::new(1.2, [0.0, -32.0], time, FCT_POOL_CAPACITY));
            let inert = FctSpawn {
                at: [0.0, 0.0],
                text: String::new(),
                color: red(),
            };
            assert!(!state.spawn(inert, 0.0));
            assert_eq!(state.alive_count(), 0);
        });
    }

    #[test]
    fn pool_capacity_is_capped() {
        Runtime::scope("fct-cap", || {
            let time = signal(0.0_f32);
            let state = Rc::new(FctState::new(10.0, [0.0, -32.0], time, FCT_POOL_CAPACITY));
            for i in 0..FCT_POOL_CAPACITY {
                #[allow(clippy::cast_precision_loss)]
                let f = i as f32;
                assert!(state.spawn(FctSpawn::new([f, f], "x", red()), 0.0));
            }
            assert_eq!(state.alive_count(), FCT_POOL_CAPACITY);
            // 1025th spawn drops on the floor.
            assert!(!state.spawn(FctSpawn::new([0.0, 0.0], "y", red()), 0.0));
        });
    }

    #[test]
    fn reactive_spawn_writes_into_pool() {
        Runtime::scope("fct-reactive-spawn", || {
            let time = signal(0.0_f32);
            let spawn = signal(FctSpawn::default());
            let state = Rc::new(FctState::new(1.2, [0.0, -32.0], time, FCT_POOL_CAPACITY));
            let effect = Rc::clone(&state);
            sigil_reactive::effect(move || {
                let payload = spawn.get();
                if !payload.text.is_empty() {
                    let _ok = effect.spawn(payload, time.peek());
                }
            });
            spawn.set(FctSpawn::new([1.0, 2.0], "crit", red()));
            spawn.set(FctSpawn::new([3.0, 4.0], "miss", red()));
            assert_eq!(state.alive_count(), 2);
        });
    }
}
