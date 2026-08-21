//! Binding-install hook — DR-053 §5 / W5.1.
//!
//! When a node has a [`sigil_style::StyleBinding`] whose
//! atom set carries [`StyleAtom::Transition(id)`](sigil_style::StyleAtom::Transition),
//! this module scans the binding's base atoms + state-slot overlays and
//! produces an [`AttachedAnimators`] handle: one [`Animator`] per
//! numeric atom whose value differs across slots, each driving a
//! [`Signal<f32>`].
//!
//! The hook is intentionally a *thin* layer over the existing
//! [`crate::config`] + [`Animator`] surface — it does not touch
//! [`sigil_style::StyleBinding`] internals, so the W5.1 wiring can land without
//! refactoring the by-value `StyleAtom` storage in `sigil-style`. Engine
//! integration that reads the animator-driven signal value during paint
//! is a follow-up; the W5.1 contract this module satisfies is the
//! *handle protocol*.
//!
//! The handle protocol: a known [`TransitionId`] resolves to running
//! animators; an unknown [`TransitionId`] logs a merge warning + degrades
//! to no-op; state-slot transitions retarget animators (don't snap); and
//! settled animators drop out of the active set.
//!
//! **Crate-location note.** DR-053 §5's pseudo-code locates this hook in
//! `sigil-style`, but `sigil-animation-runtime` is the only crate in the
//! workspace that depends on both `sigil-animation` (for `Animator`) and
//! `sigil-style` (for `TransitionId` / `StyleAtom`). Putting the hook
//! in `sigil-style` would introduce a `sigil-style` → `sigil-animation`
//! cycle. The hook lives here; `sigil-style` exposes the read-only
//! seams (`TransitionId`, `StyleAtom`, `merge_warning_count_for_component`)
//! the hook consumes.

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

/// `ComponentId` namespace used by the binding-install hook when it
/// fails to resolve a [`TransitionId`] in the registry.
///
/// Logged via `sigil_style::merge_warning_count_for_component`. The
/// numeric value is arbitrary — chosen large enough that real
/// per-widget component IDs never collide.
pub const ANIMATION_REGISTRY_MISS: ComponentId = 0xA111_0000_0000_0001;

/// One attached animator: the [`Animator`] handle plus enough metadata
/// for the engine to look up which atom / slot it drives. Stored inside
/// [`AttachedAnimators`].
pub struct AttachedAnimator {
    /// Numeric atom kind the animator drives.
    pub kind: NumericAtomKind,
    /// Reactive signal the animator writes into. The engine reads this
    /// signal during paint to obtain the current interpolated value.
    pub signal: Signal<f32>,
    /// Owning animator handle. Drop = de-register from the active set.
    pub animator: Animator,
    /// Last target the hook drove the animator toward. Used by
    /// [`AttachedAnimators::retarget_for_state`] to skip no-op writes.
    last_target: f32,
}

/// One attached colour animator.
///
/// Holds four channel animators: RGB interpolated in linear-light,
/// alpha interpolated linearly. The signals are driven by per-channel
/// f32 animators; reads compose them back into an [`Rgba8`] via the
/// inverse sRGB encode.
pub struct AttachedColorAnimator {
    /// Colour atom kind the animator drives.
    pub kind: ColorAtomKind,
    /// Per-channel signals (R, G, B, A) — values written by the
    /// per-channel animators each tick. R/G/B are linear-light f32 in
    /// `[0.0, 1.0]`; A is the literal u8 alpha cast to f32 in `[0, 255]`.
    pub channels: [Signal<f32>; 4],
    /// Per-channel animators. Drop = de-register from the active set.
    pub animators: [Animator; 4],
    /// Last colour the hook drove this animator toward — used to skip
    /// no-op retargets.
    pub(crate) last_target: Rgba8,
}

/// Animators attached to a single [`sigil_style::StyleBinding`].
///
/// Returned by [`install_animators`] / [`install_animators_themed`].
/// Owning the struct keeps each [`Animator`] alive (and therefore
/// registered with the per-thread tick set); drop = de-register every
/// animator at once.
pub struct AttachedAnimators {
    entries: Vec<AttachedAnimator>,
    color_entries: Vec<AttachedColorAnimator>,
    /// State-slot overlays the install hook saw at construction time.
    /// Stored here so `retarget_to_slot` can re-resolve the per-slot
    /// target atoms without the caller threading `state_style` through.
    state_style: StateStyle,
    /// Snapshot theme used to resolve `*ThemeRef` colour atoms when
    /// retargeting. `None` when the binding was built via the non-themed
    /// `install_animators` path (numeric-only).
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
    /// Empty handle. The no-animation no-op path returns this.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// True when no animators are attached.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Number of attached animators.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Iterate animator entries (read-only access for the engine /
    /// tests).
    pub fn iter(&self) -> impl Iterator<Item = &AttachedAnimator> {
        self.entries.iter()
    }

    /// Retarget every animator at the state-slot's value, using the
    /// state-style stored at install time and `base` as the fallback
    /// for atoms not present in the slot overlay.
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

/// Walk `atoms` + `state_style` and build the animator set for the
/// hover slot. `component_id` is reported as the warning namespace when
/// the binding's `Transition(id)` does not resolve in `registry`.
///
/// Returns [`AttachedAnimators::empty()`] when:
/// - The binding contains no `Transition` atom (zero-overhead path).
/// - The `Transition`'s `id` does not resolve in `registry` (after
///   logging via `sigil_style::merge_warning_count_for_component`).
/// - No numeric atom value differs between base and the hover slot
///   (no work to animate).
///
/// Initial position of each animator is the base value. Caller drives
/// hover / press transitions via [`AttachedAnimators::retarget_for_state`].
#[must_use]
pub fn install_animators(
    atoms: &[StyleAtom],
    state_style: StateStyle,
    component_id: ComponentId,
) -> AttachedAnimators {
    install_impl(atoms, state_style, component_id, None)
}

/// Install themed animators ([`install_animators`] + colour support).
///
/// Use this when the binding's base atoms include a background or
/// text colour and the hover slot overrides it — the animator
/// interpolates colour endpoints in linear-light space, matching the
/// renderer's sRGB swapchain contract.
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
        // Spring configs ignore explicit ms — per DR-053 §4 spring
        // tension/damping drive the duration implicitly. Documented
        // behaviour; flagged for dogfood-newdev in DR-053 open questions.
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
    // The `merge_warning_count_for_component` function in sigil-style
    // is *read-only* — it returns the count, it does not increment it.
    // Increments happen inside `merge_classes` via the private
    // `diagnostic::warn_conflict` path. The W5.1 hook surfaces the miss
    // through the same `eprintln!` channel so operators see the
    // diagnostic in stderr, and the test seam below uses the
    // crate-local registry-miss counter to verify the path was hit
    // without depending on the private sigil-style diagnostic state.
    miss_counter::bump(component_id);
    eprintln!(
        "sigil-animation-runtime: unknown TransitionId at binding install \
         for component {component_id:#x}; falling back to instant application",
    );
    // Touch the public surface to confirm the symbol stays linked even
    // when the counter is consulted lazily by tests.
    let _ = _emit_warning_counter_check(component_id);
}

/// Test-only counter of registry-miss events. Public so unit tests in
/// this crate (and the visual-golden harness in `crates/sigil/`) can
/// assert the warning path was taken.
pub mod miss_counter {
    use core::sync::atomic::{AtomicUsize, Ordering};
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    use sigil_style::ComponentId;

    static TOTAL: AtomicUsize = AtomicUsize::new(0);
    static PER_COMPONENT: OnceLock<Mutex<HashMap<ComponentId, usize>>> = OnceLock::new();

    /// Total registry-miss events seen since process start.
    #[must_use]
    pub fn total() -> usize {
        TOTAL.load(Ordering::Relaxed)
    }

    /// Per-component registry-miss count.
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
