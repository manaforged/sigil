use std::cell::RefCell;
use std::sync::OnceLock;

use smallvec::SmallVec;

use crate::animation_runtime::binding_animate::{
    AttachedAnimator, AttachedColorAnimator, find_numeric,
};
use crate::animation_runtime::binding_animate_color::{
    build_color_entry, read_color, retarget_color_entry,
};
use crate::animation_runtime::playback::{Player, animation};
use crate::animation_runtime::registry::{Transitioned, transition_config};
use crate::animation_runtime::{AnimationConfig, Animator};
use crate::kernel::{Cx, DeltaKind, NodeId, NodeTag};
use crate::reactive::{Signal, effect_named, owner_by_id, signal};
use crate::style::{
    AnimatorOverlay, ColorAtomKind, NumericAtomKind, RenderStyle, Rgba8, StyleAtom, StyleBinding,
    StyleEngine, Theme, Tone, ToneContext,
};

#[derive(Clone, Copy, Default)]
struct Targets {
    numeric: [Option<f32>; 16],
    colors: [Option<Rgba8>; 3],
}

#[derive(Default)]
struct State {
    enabled: bool,
    previous: Option<Targets>,
    numeric: Vec<AttachedAnimator>,
    colors: Vec<AttachedColorAnimator>,
    player: Option<Player>,
    inherited_text: Option<Rgba8>,
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

    fn transition_color(&self, kind: ColorAtomKind) -> Option<Rgba8> {
        if !self.enabled {
            return None;
        }
        let targeted = self
            .previous
            .is_some_and(|target| target.colors.get(kind as usize).is_some_and(Option::is_some));
        self.colors
            .iter()
            .find(|entry| entry.kind == kind)
            .filter(|entry| {
                targeted || entry.animators.iter().any(|animator| !animator.settled())
            })
            .map(read_color)
    }

    fn underlying_color(&self, kind: ColorAtomKind) -> Rgba8 {
        let target = self
            .previous
            .and_then(|target| target.colors.get(kind as usize).copied().flatten());
        let fallback = match kind {
            ColorAtomKind::Background => Rgba8 { r: 0, g: 0, b: 0, a: 0 },
            ColorAtomKind::Text | ColorAtomKind::Border => {
                self.inherited_text.unwrap_or(Rgba8 { r: 0, g: 0, b: 0, a: 0 })
            }
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

    fn reset(&mut self, previous: Targets, inherited_text: Rgba8) {
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
        if !state.enabled {
            return;
        }
        let Some(previous) = state.previous.as_ref() else {
            return;
        };
        for entry in &state.numeric {
            if !entry.kind.affects_layout()
                || previous
                    .numeric
                    .get(entry.kind as usize)
                    .is_none_or(Option::is_none)
            {
                continue;
            }
            if let Some(value) = entry.signal.try_peek() {
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
        let underlying = transitioned.or(target).unwrap_or_default();
        player.numeric(kind, underlying).or(transitioned)
    }

    fn value_for_color(&self, kind: ColorAtomKind) -> Option<Rgba8> {
        let state = self.state.borrow();
        let transitioned = state.transition_color(kind);
        let Some(player) = state.player.as_ref() else {
            return transitioned;
        };
        player
            .color(kind, state.underlying_color(kind))
            .or(transitioned)
    }

    fn update(&self, atoms: &[StyleAtom], theme: &Theme, tone: Tone, inherited_text: Rgba8) {
        let target = targets(atoms, theme, tone);
        let mut state = self.state.borrow_mut();
        state.inherited_text = Some(inherited_text);
        self.update_player(&mut state, atoms, &StyleEngine::with_tone(theme, tone));
        let Some(previous) = state.previous.replace(target) else {
            return;
        };
        let Some(config) = transition_config(atoms) else {
            state.suspend();
            return;
        };
        if !state.enabled {
            state.reset(previous, inherited_text);
        }
        state.enabled = true;
        update_numeric(&mut state.numeric, (previous, target), config, self.tag);
        update_colors(
            &mut state.colors,
            (previous, target),
            config,
            (inherited_text, self.tag),
        );
    }
}

fn update_numeric(
    entries: &mut Vec<AttachedAnimator>,
    (previous, target): (Targets, Targets),
    (config, group): Transition,
    tag: Option<NodeTag>,
) {
    for ((kind, from), to) in NumericAtomKind::ALL
        .into_iter()
        .zip(previous.numeric)
        .zip(target.numeric)
    {
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
    (previous, target): (Targets, Targets),
    (config, group): Transition,
    (inherited_text, tag): (Rgba8, Option<NodeTag>),
) {
    for ((kind, from), to) in ColorAtomKind::ALL
        .into_iter()
        .zip(previous.colors)
        .zip(target.colors)
    {
        if from == to {
            continue;
        }
        let Some(color) = to.or(from) else {
            continue;
        };
        let fallback = color_fallback(kind, color, inherited_text);
        let to = to.unwrap_or(fallback);
        let existing = entries.iter_mut().find(|entry| entry.kind == kind);
        if !group.colors() {
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

fn snap_color(entry: &mut AttachedColorAnimator, to: Rgba8) {
    retarget_color_entry(entry, to);
    for animator in &mut entry.animators {
        animator.finish();
        animator.cancel();
    }
}

const fn color_fallback(kind: ColorAtomKind, color: Rgba8, inherited_text: Rgba8) -> Rgba8 {
    match kind {
        ColorAtomKind::Text | ColorAtomKind::Border => inherited_text,
        ColorAtomKind::Background => Rgba8 { a: 0, ..color },
    }
}

fn targets(atoms: &[StyleAtom], theme: &Theme, tone: Tone) -> Targets {
    let mut render = RenderStyle::default();
    StyleEngine::with_tone(theme, tone).apply(atoms, &mut render);
    Targets {
        numeric: NumericAtomKind::ALL.map(|kind| match kind {
            NumericAtomKind::Opacity => Some(f32::from(render.opacity)),
            NumericAtomKind::BorderRadius => Some(f32::from(render.border_radius.unwrap_or(0))),
            _ => render
                .transform
                .value(kind)
                .or_else(|| find_numeric(kind, atoms)),
        }),
        colors: [
            render.background_color,
            render.text_color,
            render.border_color,
        ],
    }
}

pub(crate) fn prepare(cx: &Cx, id: NodeId) {
    static DEFAULT_THEME: OnceLock<Theme> = OnceLock::new();
    let Some(node) = cx.get(id) else {
        return;
    };
    let Some(binding) = node.get_extension::<StyleBinding>() else {
        return;
    };
    if binding.animator_overlay().is_none() && !binding.has_transition() {
        return;
    }
    let theme = std::iter::successors(Some(id), |&at| cx.parent(at))
        .find_map(|ancestor| cx.get(ancestor)?.get_extension::<Theme>())
        .unwrap_or_else(|| DEFAULT_THEME.get_or_init(Theme::default));
    let tone = node
        .get_extension::<ToneContext>()
        .copied()
        .unwrap_or_default()
        .tone;
    let inherited_text = inherited_text(cx, id, theme);
    owner_by_id(node.owner, || {
        binding.update_animations(theme, tone, inherited_text, node.tag());
    });
}

fn inherited_text(cx: &Cx, target: NodeId, theme: &Theme) -> Rgba8 {
    std::iter::successors(cx.parent(target), |&id| cx.parent(id))
        .find_map(|id| crate::style::render_style_for(id, cx.get(id)?, theme, None).text_color)
        .unwrap_or(theme.on_surface)
}
