use std::rc::Rc;

use sigil_kernel::{AccessibleName, Cx, IntoNode, NodeBuilder, fragment};
use sigil_reactive::{Signal, effect_named};
use sigil_style::{NodeStyleExt, Style, StyleAccess};

use super::super::shared::BuilderMutator;
use super::spawn::FctSpawn;
use super::state::{FctSettings, FctState};
use super::style::FloaterStyle;

pub struct FloatingTextBuilder {
    spawn: Signal<FctSpawn>,
    settings: FctSettings,
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
}

impl FloatingTextBuilder {
    #[must_use]
    pub fn new(spawn: Signal<FctSpawn>) -> Self {
        Self {
            spawn,
            settings: FctSettings::new(),
            mutators: Vec::new(),
            style_accumulator: Style::new(),
        }
    }

    #[must_use]
    pub const fn duration(mut self, seconds: f32) -> Self {
        self.settings.duration = if seconds > f32::EPSILON {
            seconds
        } else {
            f32::EPSILON
        };
        self
    }

    #[must_use]
    pub const fn drift(mut self, drift: [f32; 2]) -> Self {
        self.settings.drift = drift;
        self
    }

    #[must_use]
    pub const fn time(mut self, time: Signal<f32>) -> Self {
        self.settings.time = Some(time);
        self
    }

    #[must_use]
    pub const fn capacity(mut self, capacity: usize) -> Self {
        self.settings.capacity = capacity;
        self
    }

    #[must_use]
    pub const fn at(mut self, pos: Signal<(f32, f32)>) -> Self {
        self.settings.anchor = Some(pos);
        self
    }

    #[must_use]
    pub const fn style(mut self, style: FloaterStyle) -> Self {
        self.settings.style = Some(style);
        self
    }

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
            .unwrap_or_else(|| sigil_reactive::signal(0.0_f32));
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

impl IntoNode for FloatingTextBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

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

#[derive(Clone, Debug)]
pub struct FctNode(pub Rc<FctState>);

impl FctNode {
    pub const fn state(&self) -> &Rc<FctState> {
        &self.0
    }
}
