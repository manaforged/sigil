use std::cell::RefCell;
use std::hash::{Hash as _, Hasher as _};

use ahash::AHasher;
use smallvec::SmallVec;

use crate::animation::runtime::binding_animate::{AttachedAnimator, AttachedColorAnimator};
use crate::animation::runtime::binding_animate_color::{
    build_color_entry, read_color, retarget_color_entry,
};
use crate::animation::runtime::playback::{Player, animation};
use crate::animation::runtime::registry::{Transitioned, transition_config};
use crate::animation::runtime::targets::{Change, Targets, targets};
use crate::animation::runtime::{AnimationConfig, Animator};
use crate::kernel::{Cx, DeltaKind, NodeId, NodeTag};
use crate::reactive::{Runtime, Signal, effect_named, owner_by_id, signal};
use crate::style::{
    AnimatorOverlay, Color, ColorAtomKind, NumericAtomKind, StyleAtom, StyleBinding, StyleEngine,
    Theme, Tone, ToneContext,
};

const CLEAR: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 0,
};

#[derive(Default)]
struct State {
    enabled: bool,
    previous: Option<Targets>,
    seed: Option<u64>,
    numeric: Vec<AttachedAnimator>,
    colors: Vec<AttachedColorAnimator>,
    player: Option<Player>,
    inherited_text: Option<Color>,
}

type Transition = (AnimationConfig, Transitioned);

impl State {
    fn transition_value(&self, kind: NumericAtomKind) -> Option<f32> {
        if !self.enabled || self.previous?.numeric.get(kind as usize)?.is_none() {
            return None;
        }
        self.numeric
            .iter()
            .find(|entry| entry.kind == kind)
            .and_then(|entry| entry.signal.try_peek())
    }

    fn transition_color(&self, kind: ColorAtomKind) -> Option<Color> {
        if !self.enabled {
            return None;
        }
        let targeted = self.previous.is_some_and(|target| {
            target
                .colors
                .get(kind as usize)
                .is_some_and(Option::is_some)
        });
        self.colors
            .iter()
            .find(|entry| entry.kind == kind)
            .filter(|entry| targeted || entry.animators.iter().any(|animator| !animator.settled()))
            .map(read_color)
    }

    fn underlying_color(&self, kind: ColorAtomKind) -> Color {
        let target = self
            .previous
            .and_then(|target| target.colors.get(kind as usize).copied().flatten());
        let fallback = match kind {
            ColorAtomKind::Text | ColorAtomKind::Border => self.inherited_text.unwrap_or(CLEAR),
            _ => CLEAR,
        };
        self.transition_color(kind).or(target).unwrap_or(fallback)
    }

    fn suspend(&mut self) {
        self.enabled = false;
        for entry in &mut self.numeric {
            entry.animator.cancel();
        }
        for entry in &mut self.colors {
            for animator in &mut entry.animators {
                animator.cancel();
            }
        }
    }

    fn reset(&mut self, previous: &Targets, inherited_text: Color) {
        for entry in &mut self.numeric {
            if let Some(Some(value)) = previous.numeric.get(entry.kind as usize) {
                entry.signal.try_set(*value);
            }
        }
        for entry in &mut self.colors {
            let value = previous
                .colors
                .get(entry.kind as usize)
                .copied()
                .flatten()
                .unwrap_or_else(|| color_fallback(entry.kind, entry.last_target, inherited_text));
            entry.last_target = read_color(entry);
            retarget_color_entry(entry, value);
            for animator in &mut entry.animators {
                animator.finish();
                animator.cancel();
            }
        }
    }
}

pub(crate) struct StyleAnimator {
    state: RefCell<State>,
    tag: Option<NodeTag>,
}

impl StyleAnimator {
    pub(crate) fn new(atoms: &[StyleAtom], tag: Option<NodeTag>) -> Option<Self> {
        if transition_config(atoms).is_none() && animation(atoms).is_none() {
            return None;
        }
        Some(Self {
            state: RefCell::new(State::default()),
            tag,
        })
    }

    fn update_player(&self, state: &mut State, atoms: &[StyleAtom], engine: &StyleEngine<'_>) {
        if state.player.is_none() && animation(atoms).is_none() {
            return;
        }
        let player = state.player.get_or_insert_with(|| {
            let player = Player::new();
            record_on(self.tag, &[player.time()], DeltaKind::Paint);
            player
        });
        player.update(atoms, engine);
    }
}

fn record_on(tag: Option<NodeTag>, signals: &[Signal<f32>], kind: DeltaKind) {
    let Some(tag) = tag else {
        return;
    };
    let signals: SmallVec<[Signal<f32>; 4]> = signals.iter().copied().collect();
    effect_named("sigil-animation::style_delta", move || {
        for signal in &signals {
            signal.track();
        }
        tag.record(kind);
    });
}

impl AnimatorOverlay for StyleAnimator {
    fn visit_layout_values(&self, visit: &mut dyn FnMut(NumericAtomKind, f32)) {
        let state = self.state.borrow();
        if state.player.is_some() {
            drop(state);
            crate::style::animator_overlay::default_visit_layout_values(self, visit);
            return;
        }
        if !state.enabled {
            return;
        }
        let Some(previous) = state.previous.as_ref() else {
            return;
        };
        for entry in &state.numeric {
            if entry.kind.affects_layout()
                && previous
                    .numeric
                    .get(entry.kind as usize)
                    .is_some_and(Option::is_some)
                && let Some(value) = entry.signal.try_peek()
            {
                visit(entry.kind, value);
            }
        }
    }

    fn value_for(&self, kind: NumericAtomKind) -> Option<f32> {
        let state = self.state.borrow();
        let transitioned = state.transition_value(kind);
        let Some(player) = state.player.as_ref() else {
            return transitioned;
        };
        let target = state
            .previous
            .and_then(|target| target.numeric.get(kind as usize).copied().flatten());
        let underlying = transitioned.or(target).unwrap_or_else(|| kind.initial());
        player.numeric(kind, underlying).or(transitioned)
    }

    fn value_for_color(&self, kind: ColorAtomKind) -> Option<Color> {
        let state = self.state.borrow();
        let transitioned = state.transition_color(kind);
        let Some(player) = state.player.as_ref() else {
            return transitioned;
        };
        player
            .color(kind, state.underlying_color(kind))
            .or(transitioned)
    }

    fn cache_key(&self) -> u64 {
        let state = self.state.borrow();
        if state.player.is_some() {
            drop(state);
            return crate::style::animator_overlay::default_cache_key(self);
        }
        if !state.enabled || (state.numeric.is_empty() && state.colors.is_empty()) {
            return 0;
        }
        transition_cache_key(&state)
    }

    fn update(&self, atoms: &[StyleAtom], theme: &Theme, tone: Tone, inherited_text: Color) {
        let target = targets(atoms, theme, tone);
        let mut state = self.state.borrow_mut();
        state.inherited_text = Some(inherited_text);
        self.update_player(&mut state, atoms, &StyleEngine::with_tone(theme, tone));
        let Some(previous) = state.previous else {
            state.previous = Some(target);
            state.seed = Runtime::layout_paints();
            return;
        };
        if state.seed.is_some() && state.seed == Runtime::layout_paints() {
            state.previous = Some(target);
            return;
        }
        state.seed = None;
        let change = Change::new(&previous, &target);
        state.previous = Some(change.to);
        let Some(config) = transition_config(atoms) else {
            state.suspend();
            return;
        };
        if !state.enabled {
            state.reset(&previous, inherited_text);
        }
        state.enabled = true;
        update_numeric(&mut state.numeric, &change, config, self.tag);
        update_colors(
            &mut state.colors,
            &change,
            config,
            (inherited_text, self.tag),
        );
    }
}

fn update_numeric(
    entries: &mut Vec<AttachedAnimator>,
    change: &Change,
    (config, group): Transition,
    tag: Option<NodeTag>,
) {
    for kind in NumericAtomKind::ALL {
        let (from, to) = change.numeric(kind);
        let (Some(from), Some(to)) = (from.filter(|_| group.numeric(kind)), to) else {
            if let Some(entry) = entries.iter_mut().find(|entry| entry.kind == kind) {
                entry.animator.cancel();
                if let Some(to) = to {
                    entry.signal.try_set(to);
                }
            }
            continue;
        };
        if from.to_bits() == to.to_bits() {
            continue;
        }
        if let Some(entry) = entries.iter_mut().find(|entry| entry.kind == kind) {
            entry.animator.set_config(config);
            entry.animator.target(to);
        } else {
            let value = signal(from);
            record_on(tag, &[value], DeltaKind::layout_if(kind.affects_layout()));
            let mut animator = Animator::new(value, config);
            animator.target(to);
            entries.push(AttachedAnimator {
                kind,
                signal: value,
                animator,
            });
        }
    }
}

fn update_colors(
    entries: &mut Vec<AttachedColorAnimator>,
    change: &Change,
    (config, group): Transition,
    (inherited_text, tag): (Color, Option<NodeTag>),
) {
    for kind in ColorAtomKind::ALL {
        let (from, to) = change.color(kind);
        if from == to {
            continue;
        }
        let Some(color) = to.or(from) else {
            continue;
        };
        let fallback = color_fallback(kind, color, inherited_text);
        let to = to.unwrap_or(fallback);
        let existing = entries.iter_mut().find(|entry| entry.kind == kind);
        if !group.color(kind) || change.discrete(kind.family()) {
            if let Some(entry) = existing {
                snap_color(entry, to);
            }
            continue;
        }
        if let Some(entry) = existing {
            for animator in &mut entry.animators {
                animator.set_config(config);
            }
            retarget_color_entry(entry, to);
        } else {
            let mut entry = build_color_entry(kind, from.unwrap_or(fallback), config);
            record_on(tag, &entry.channels, DeltaKind::Paint);
            retarget_color_entry(&mut entry, to);
            entries.push(entry);
        }
    }
}

fn snap_color(entry: &mut AttachedColorAnimator, to: Color) {
    retarget_color_entry(entry, to);
    for animator in &mut entry.animators {
        animator.finish();
        animator.cancel();
    }
}

const fn color_fallback(kind: ColorAtomKind, color: Color, inherited_text: Color) -> Color {
    match kind {
        ColorAtomKind::Text | ColorAtomKind::Border => inherited_text,
        _ => Color { a: 0, ..color },
    }
}

pub(crate) fn prepare(cx: &Cx, id: NodeId) {
    let Some(node) = cx.get(id) else {
        return;
    };
    let Some(binding) = node.extension::<StyleBinding>() else {
        return;
    };
    if binding.animator_overlay().is_none() && !binding.has_transition() {
        return;
    }
    let theme = std::iter::successors(Some(id), |&at| cx.parent(at))
        .find_map(|ancestor| cx.get(ancestor)?.extension::<Theme>())
        .unwrap_or_else(crate::style::theme::fallback_theme);
    let tone = node
        .extension::<ToneContext>()
        .copied()
        .unwrap_or_default()
        .tone;
    let inherited_text = inherited_text(cx, id, theme);
    owner_by_id(node.owner, || {
        binding.update_animations(theme, tone, inherited_text, node.tag());
    });
}

fn transition_cache_key(state: &State) -> u64 {
    let Some(previous) = state.previous.as_ref() else {
        return 0;
    };
    let mut hasher = AHasher::default();
    let mut active = false;
    for entry in &state.numeric {
        if previous
            .numeric
            .get(entry.kind as usize)
            .is_some_and(Option::is_some)
            && let Some(value) = entry.signal.try_peek()
        {
            entry.kind.hash(&mut hasher);
            value.to_bits().hash(&mut hasher);
            active = true;
        }
    }
    for entry in &state.colors {
        let targeted = previous
            .colors
            .get(entry.kind as usize)
            .is_some_and(Option::is_some);
        if targeted || entry.animators.iter().any(|animator| !animator.settled()) {
            entry.kind.hash(&mut hasher);
            read_color(entry).hash(&mut hasher);
            active = true;
        }
    }
    if active { hasher.finish() } else { 0 }
}

fn inherited_text(cx: &Cx, target: NodeId, theme: &Theme) -> Color {
    std::iter::successors(cx.parent(target), |&id| cx.parent(id))
        .find_map(|id| crate::style::render_style_for(cx.get(id)?, theme, None).text_color)
        .unwrap_or(theme.on_surface)
}
