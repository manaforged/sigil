use std::cell::RefCell;
use std::sync::OnceLock;

use crate::animation_runtime::binding_animate::{
    AttachedAnimator, AttachedColorAnimator, find_numeric,
};
use crate::animation_runtime::binding_animate_color::{
    build_color_entry, read_color, retarget_color_entry,
};
use crate::animation_runtime::{AnimationConfig, Animator, config};
use crate::kernel::{Cx, NodeId};
use crate::reactive::{owner_by_id, signal};
use crate::style::{
    AnimatorOverlay, ColorAtomKind, NumericAtomKind, RenderStyle, Rgba8, StateSlot, StyleAtom,
    StyleBinding, StyleEngine, Theme, Tone, ToneContext,
};

#[derive(Clone, Copy, Default)]
struct Targets {
    numeric: [Option<f32>; 14],
    colors: [Option<Rgba8>; 3],
}

#[derive(Default)]
struct State {
    enabled: bool,
    previous: Option<Targets>,
    numeric: Vec<AttachedAnimator>,
    colors: Vec<AttachedColorAnimator>,
}

impl State {
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
}

impl StyleAnimator {
    pub(crate) fn new(atoms: &[StyleAtom]) -> Option<Self> {
        transition_config(atoms)?;
        Some(Self {
            state: RefCell::new(State::default()),
        })
    }
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
        if !state.enabled || state.previous?.numeric.get(kind as usize)?.is_none() {
            return None;
        }
        state
            .numeric
            .iter()
            .find(|entry| entry.kind == kind)
            .and_then(|entry| entry.signal.try_peek())
    }

    fn value_for_color(&self, kind: ColorAtomKind) -> Option<Rgba8> {
        let state = self.state.borrow();
        if !state.enabled {
            return None;
        }
        state
            .colors
            .iter()
            .find(|entry| entry.kind == kind)
            .filter(|entry| {
                state.previous.is_some_and(|target| {
                    target
                        .colors
                        .get(kind as usize)
                        .is_some_and(Option::is_some)
                }) || entry.animators.iter().any(|animator| !animator.settled())
            })
            .map(read_color)
    }

    fn retarget_to_slot(&mut self, _slot: StateSlot, _base: &[StyleAtom]) {}

    fn update(&self, atoms: &[StyleAtom], theme: &Theme, tone: Tone, inherited_text: Rgba8) {
        let target = targets(atoms, theme, tone);
        let mut state = self.state.borrow_mut();
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
        update_numeric(&mut state.numeric, previous, target, config);
        update_colors(&mut state.colors, previous, target, config, inherited_text);
    }
}

fn update_numeric(
    entries: &mut Vec<AttachedAnimator>,
    previous: Targets,
    target: Targets,
    config: AnimationConfig,
) {
    for ((kind, from), to) in NumericAtomKind::ALL
        .into_iter()
        .zip(previous.numeric)
        .zip(target.numeric)
    {
        let (Some(from), Some(to)) = (from, to) else {
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
            let mut animator = Animator::new(value, config);
            animator.target(to);
            entries.push(AttachedAnimator {
                kind,
                signal: value,
                animator,
                last_target: to,
            });
        }
    }
}

fn update_colors(
    entries: &mut Vec<AttachedColorAnimator>,
    previous: Targets,
    target: Targets,
    config: AnimationConfig,
    inherited_text: Rgba8,
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
        if let Some(entry) = entries.iter_mut().find(|entry| entry.kind == kind) {
            for animator in &mut entry.animators {
                animator.set_config(config);
            }
            retarget_color_entry(entry, to);
        } else {
            let mut entry = build_color_entry(kind, from.unwrap_or(fallback), config);
            retarget_color_entry(&mut entry, to);
            entries.push(entry);
        }
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
            NumericAtomKind::BorderRadius => {
                Some(f32::from(render.border_radius.map_or(0, |px| px.0)))
            }
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
    let mut current = Some(id);
    let mut theme = None;
    while let Some(ancestor) = current {
        theme = cx
            .get(ancestor)
            .and_then(|parent| parent.get_extension::<Theme>());
        if theme.is_some() {
            break;
        }
        current = cx.parent(ancestor);
    }
    let theme = theme.unwrap_or_else(|| DEFAULT_THEME.get_or_init(Theme::default));
    let tone = node
        .get_extension::<ToneContext>()
        .copied()
        .unwrap_or_default()
        .tone;
    let inherited_text = inherited_text(cx, id, theme);
    owner_by_id(node.owner, || {
        binding.update_animations(theme, tone, inherited_text);
    });
}

fn inherited_text(cx: &Cx, target: NodeId, theme: &Theme) -> Rgba8 {
    let mut current = cx.parent(target);
    while let Some(id) = current {
        if let Some(node) = cx.get(id)
            && let Some(color) = crate::style::render_style_for(id, node, theme, None).text_color
        {
            return color;
        }
        current = cx.parent(id);
    }
    theme.on_surface
}

fn transition_config(atoms: &[StyleAtom]) -> Option<AnimationConfig> {
    let id = atoms.iter().rev().find_map(|atom| match atom {
        StyleAtom::Transition(id) => Some(*id),
        _ => None,
    })?;
    let mut config = config(id)?;
    if let AnimationConfig::Tween { ms, .. } = &mut config
        && let Some(duration) = atoms.iter().rev().find_map(|atom| match atom {
            StyleAtom::TransitionDuration(duration) => Some(duration.0),
            _ => None,
        })
    {
        *ms = duration;
    }
    Some(config)
}
