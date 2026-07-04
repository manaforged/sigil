//! Floating-text builder — collects per-widget tuning (duration,
//! drift, clock, capacity, anchor, style) and wires the reactive spawn
//! + tick effects that drive [`FctState`].

use std::rc::Rc;

use kernel::{AccessibleName, Cx, IntoNode, NodeBuilder, fragment};
use reactive::{Signal, effect_named};
use style::{NodeStyleExt, StateHost, Style, StyleAccess};

use super::super::shared::BuilderMutator;
use super::spawn::FctSpawn;
use super::state::{FctSettings, FctState};
use super::style::FloaterStyle;

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
    /// `0.0` time signal, and [`super::state::FCT_POOL_CAPACITY`] slots.
    #[must_use]
    pub fn new(spawn: Signal<FctSpawn>) -> Self {
        Self {
            spawn,
            settings: FctSettings::new(),
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
        self.settings.time = Some(time);
        self
    }

    /// Set the maximum number of simultaneous particles.
    #[must_use]
    pub const fn capacity(mut self, capacity: usize) -> Self {
        self.settings.capacity = capacity;
        self
    }

    /// Anchor spawned particles to a caller-provided screen position
    /// that may change over the particle's lifetime — e.g. a unit's
    /// projected world-to-screen coordinate as it walks. While set,
    /// every live particle's position is recomputed each tick from
    /// `pos`'s current value plus the elapsed-time drift offset,
    /// instead of the fixed spawn-time origin. Sigil never touches the
    /// projection math itself — the caller supplies the screen point.
    #[must_use]
    pub const fn at(mut self, pos: Signal<(f32, f32)>) -> Self {
        self.settings.anchor = Some(pos);
        self
    }

    /// Set a semantic color intent (see [`FloaterStyle`]), overriding
    /// every spawned particle's color with the resolved theme token in
    /// place of whatever [`FctSpawn::color`] the caller supplied.
    #[must_use]
    pub const fn style(mut self, style: FloaterStyle) -> Self {
        self.settings.style = Some(style);
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
        let time = self
            .settings
            .time
            .unwrap_or_else(|| reactive::signal(0.0_f32));
        let state = Rc::new(FctState::from_settings(self.settings, time));
        let extension = Rc::clone(&state);

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
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl StateHost for FloatingTextBuilder {}

impl IntoNode for FloatingTextBuilder {
    fn build(self, cx: &mut Cx) -> kernel::NodeId {
        self.into_node_builder().build(cx)
    }
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
    pub const fn state(&self) -> &Rc<FctState> {
        &self.0
    }
}
