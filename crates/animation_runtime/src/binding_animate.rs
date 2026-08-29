use sigil_reactive::{Signal, signal};
use sigil_style::{
    AnimatorOverlay, ColorAtomKind, ComponentId, MillisU16, NumericAtomKind, Rgba8, StateSlot,
    StateStyle, StyleAtom, Theme, TransitionId,
    merge_warning_count_for_component as _emit_warning_counter_check,
};

use crate::binding_animate_color::{
    ThemeSnapshot, build_color_entry, find_color, read_color, retarget_color_entry,
};
use crate::{AnimationConfig, Animator};

pub const ANIMATION_REGISTRY_MISS: ComponentId = 0xA111_0000_0000_0001;

pub struct AttachedAnimator {
    pub kind: NumericAtomKind,
    pub signal: Signal<f32>,
    pub animator: Animator,
    last_target: f32,
}

pub struct AttachedColorAnimator {
    pub kind: ColorAtomKind,
    pub channels: [Signal<f32>; 4],
    pub animators: [Animator; 4],
    pub(crate) last_target: Rgba8,
}

pub struct AttachedAnimators {
    entries: Vec<AttachedAnimator>,
    color_entries: Vec<AttachedColorAnimator>,
    state_style: StateStyle,
    theme_snapshot: Option<ThemeSnapshot>,
}

impl core::fmt::Debug for AttachedAnimators {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AttachedAnimators")
            .field("entries", &self.entries.len())
            .field("color_entries", &self.color_entries.len())
            .field("has_theme_snapshot", &self.theme_snapshot.is_some())
            .finish_non_exhaustive()
    }
}

impl Default for AttachedAnimators {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            color_entries: Vec::new(),
            state_style: StateStyle::EMPTY,
            theme_snapshot: None,
        }
    }
}

impl AttachedAnimators {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &AttachedAnimator> {
        self.entries.iter()
    }

    pub fn retarget_for_state(&mut self, slot: StateSlot, base: &[StyleAtom]) {
        let slot_atoms = slot_atoms(self.state_style, slot);
        for entry in &mut self.entries {
            let target = resolve_numeric(entry.kind, base, slot_atoms);
            if (target - entry.last_target).abs() < f32::EPSILON {
                continue;
            }
            entry.last_target = target;
            entry.animator.target(target);
        }
        if let Some(theme) = self.theme_snapshot.as_ref() {
            for entry in &mut self.color_entries {
                let target = find_color(entry.kind, slot_atoms, theme)
                    .or_else(|| find_color(entry.kind, base, theme));
                if let Some(target) = target {
                    retarget_color_entry(entry, target);
                }
            }
        }
    }
}

impl AnimatorOverlay for AttachedAnimators {
    fn value_for(&self, kind: NumericAtomKind) -> Option<f32> {
        self.entries
            .iter()
            .find(|entry| entry.kind == kind)
            .map(|entry| entry.signal.peek())
    }

    fn value_for_color(&self, kind: ColorAtomKind) -> Option<Rgba8> {
        self.color_entries
            .iter()
            .find(|entry| entry.kind == kind)
            .map(read_color)
    }

    fn retarget_to_slot(&mut self, slot: StateSlot, base: &[StyleAtom]) {
        self.retarget_for_state(slot, base);
    }
}

#[must_use]
pub fn install_animators(
    atoms: &[StyleAtom],
    state_style: StateStyle,
    component_id: ComponentId,
) -> AttachedAnimators {
    install_impl(atoms, state_style, component_id, None)
}

#[must_use]
pub fn install_animators_themed(
    atoms: &[StyleAtom],
    state_style: StateStyle,
    component_id: ComponentId,
    theme: &Theme,
) -> AttachedAnimators {
    install_impl(
        atoms,
        state_style,
        component_id,
        Some(ThemeSnapshot::from_theme(theme)),
    )
}

#[allow(
    clippy::large_types_passed_by_value,
    reason = "`theme_snapshot` is consumed — install_impl stores it into the \
              returned AttachedAnimators. Passing by reference \
              would force a clone at the assignment site."
)]
fn install_impl(
    atoms: &[StyleAtom],
    state_style: StateStyle,
    component_id: ComponentId,
    theme_snapshot: Option<ThemeSnapshot>,
) -> AttachedAnimators {
    let Some(id) = first_transition_id(atoms) else {
        return AttachedAnimators::empty();
    };
    let Some(config) = crate::registry::config(id) else {
        emit_registry_miss(component_id);
        return AttachedAnimators::empty();
    };
    let config = apply_duration_override(config, atoms);

    let hover_atoms = state_style.hovered();
    let mut entries = Vec::new();
    for kind in ANIMATABLE_KINDS {
        let base_value = find_numeric(*kind, atoms);
        let hover_value = find_numeric(*kind, hover_atoms);
        let (Some(base), Some(hover)) = (base_value, hover_value) else {
            continue;
        };
        if (base - hover).abs() < f32::EPSILON {
            continue;
        }
        entries.push(build_entry(*kind, base, config));
    }

    let mut color_entries = Vec::new();
    if let Some(theme) = theme_snapshot.as_ref() {
        for kind in [ColorAtomKind::Background, ColorAtomKind::Text] {
            let base_value = find_color(kind, atoms, theme);
            let hover_value = find_color(kind, hover_atoms, theme);
            let (Some(base), Some(hover)) = (base_value, hover_value) else {
                continue;
            };
            if base == hover {
                continue;
            }
            color_entries.push(build_color_entry(kind, base, config));
        }
    }

    AttachedAnimators {
        entries,
        color_entries,
        state_style,
        theme_snapshot,
    }
}

const ANIMATABLE_KINDS: &[NumericAtomKind] = &[
    NumericAtomKind::Opacity,
    NumericAtomKind::BorderRadius,
    NumericAtomKind::FontSize,
    NumericAtomKind::LineHeight,
    NumericAtomKind::Gap,
    NumericAtomKind::PaddingX,
    NumericAtomKind::PaddingY,
];

fn build_entry(kind: NumericAtomKind, initial: f32, config: AnimationConfig) -> AttachedAnimator {
    let bound = signal(initial);
    let animator = Animator::new(bound, config);
    AttachedAnimator {
        kind,
        signal: bound,
        animator,
        last_target: initial,
    }
}

fn first_transition_id(atoms: &[StyleAtom]) -> Option<TransitionId> {
    atoms.iter().find_map(|atom| match *atom {
        StyleAtom::Transition(id) => Some(id),
        _ => None,
    })
}

fn first_transition_duration(atoms: &[StyleAtom]) -> Option<MillisU16> {
    atoms.iter().find_map(|atom| match *atom {
        StyleAtom::TransitionDuration(ms) => Some(ms),
        _ => None,
    })
}

fn apply_duration_override(config: AnimationConfig, atoms: &[StyleAtom]) -> AnimationConfig {
    let Some(ms) = first_transition_duration(atoms) else {
        return config;
    };
    match config {
        AnimationConfig::Tween { easing, .. } => AnimationConfig::Tween { ms: ms.0, easing },
        AnimationConfig::Spring { .. } => config,
    }
}

const fn slot_atoms(state_style: StateStyle, slot: StateSlot) -> &'static [StyleAtom] {
    match slot {
        StateSlot::Rest => state_style.rest(),
        StateSlot::Focused => state_style.focused(),
        StateSlot::Hovered => state_style.hovered(),
        StateSlot::Pressed => state_style.pressed(),
        StateSlot::Disabled => state_style.disabled(),
    }
}

fn resolve_numeric(kind: NumericAtomKind, base: &[StyleAtom], overlay: &[StyleAtom]) -> f32 {
    find_numeric(kind, overlay)
        .or_else(|| find_numeric(kind, base))
        .unwrap_or(0.0)
}

fn find_numeric(kind: NumericAtomKind, atoms: &[StyleAtom]) -> Option<f32> {
    atoms.iter().rev().find_map(|atom| match (*atom, kind) {
        (StyleAtom::Opacity(v), NumericAtomKind::Opacity) => Some(f32::from(v)),
        (StyleAtom::BorderRadius(px), NumericAtomKind::BorderRadius)
        | (StyleAtom::FontSize(px), NumericAtomKind::FontSize)
        | (StyleAtom::LineHeight(px), NumericAtomKind::LineHeight)
        | (StyleAtom::Gap(px), NumericAtomKind::Gap)
        | (StyleAtom::PaddingX(px), NumericAtomKind::PaddingX)
        | (StyleAtom::PaddingY(px), NumericAtomKind::PaddingY) => Some(f32::from(px.0)),
        _ => None,
    })
}

fn emit_registry_miss(component_id: ComponentId) {
    miss_counter::bump(component_id);
    eprintln!(
        "sigil-animation-runtime: unknown TransitionId at binding install \
         for component {component_id:#x}; falling back to instant application",
    );
    let _ = _emit_warning_counter_check(component_id);
}

pub mod miss_counter {
    use core::sync::atomic::{AtomicUsize, Ordering};
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    use sigil_style::ComponentId;

    static TOTAL: AtomicUsize = AtomicUsize::new(0);
    static PER_COMPONENT: OnceLock<Mutex<HashMap<ComponentId, usize>>> = OnceLock::new();

    #[must_use]
    pub fn total() -> usize {
        TOTAL.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn for_component(component_id: ComponentId) -> usize {
        let map = PER_COMPONENT.get_or_init(|| Mutex::new(HashMap::new()));
        let Ok(map) = map.lock() else {
            return 0;
        };
        map.get(&component_id).copied().unwrap_or(0)
    }

    pub(super) fn bump(component_id: ComponentId) {
        TOTAL.fetch_add(1, Ordering::Relaxed);
        let map = PER_COMPONENT.get_or_init(|| Mutex::new(HashMap::new()));
        if let Ok(mut map) = map.lock() {
            *map.entry(component_id).or_insert(0) += 1;
        }
    }
}

#[cfg(test)]
#[path = "binding_animate_tests.rs"]
mod tests;
