---
id: DR-053
status: proposed
date: 2026-05-18
covers: G9 animation runtime — orchestration layer over sigil-animation primitives, integration with Runtime begin_paint phase, and consumption of DR-049 TransitionId atoms
depends_on: [DR-040, DR-045, DR-047, DR-048, DR-049, DR-050]
relates: docs/plans/2026-05-16-gpui-parity-integration.md, docs/specs/g9-animation-api-sketch.md
---

# DR-053 — G9 Animation Runtime: Signal-Driven Animator + Style Transition Wiring

**Status:** Proposed
**Date:** 2026-05-18
**Covers:** The W4.G9 keystone — an animation runtime that ticks
`sigil-animation`'s `Spring` / `Tween` primitives against the per-frame
`begin_paint` boundary, drives `Signal<T>` outputs that the reactive
graph reads, and consumes the `StyleAtom::Transition` / `TransitionDuration`
atoms that `Styled::transition` already emits per DR-049.
**Depends on:**
- DR-040 (thesis lock — animation is one of GPUI's six pillars; mirror
  its shape: spring physics + duration tweens + named presets).
- DR-045 (`Entity<T>` — animations are owned by an entity's reactive
  owner; eviction on owner dispose).
- DR-047 (DrawPhase — animator ticks during `begin_paint` so the
  resulting `Signal<T>` write is observed by the *next* frame's Effect
  phase, never mid-paint).
- DR-048 (G5 `Task<T>` — animator borrows the executor-install seam to
  schedule reduced-motion fallbacks and timeline sequences).
- DR-049 (`StyleAtom::Transition(TransitionId)` + `TransitionDuration`
  — already emitted by `Styled::transition`; this DR locks the consumer).
- DR-050 (a11y — reduced-motion respects `accesskit::Live` regions and a
  global `AccessibilityPrefs::reduced_motion` signal).
**Unlocks:**
- Hover/focus/pressed slot transitions actually animate instead of
  snapping (the current behaviour with DR-049 base atoms landed).
- Wave 5 widget catalog (~21 widgets) — every state-slot transition
  inherits the same physically-grounded settle behaviour without
  per-widget plumbing.
- W4.G8 drag-and-drop drop-target highlight + W4.G7 IME candidate
  window slide-in both compose on the same animator surface.

---

## Context

`sigil-animation` already ships three primitives (DR ratified via the
existing crate, not a prior DR):

- `Spring` — Hooke's-law spring with stiffness/damping/mass, settled
  detection on both error and velocity (semi-implicit Euler integration
  per the comment in `crates/animation/src/spring.rs:1`).
- `Easing` — named curves (linear / quad / cubic / expo / back / bounce
  in/out/inOut variants).
- `Tween` — fixed-duration interpolation between two `f32` endpoints
  under an `Easing`.

The primitives are pure math. They do not own a frame loop, do not
touch `Signal<T>`, and do not drive widgets by themselves. Every
existing consumer is expected to feed `dt` per frame and read the next
value. Five things are missing for GPUI-parity animation ergonomics:

1. **A reactive seam.** Widgets author state via `Signal<T>`. Animations
   need to *drive* a `Signal<f32>` (or vector-valued signal) so the
   reactive graph re-renders on the next frame without the widget
   knowing the spring exists.
2. **A frame-tick owner.** Who calls `Spring::step(dt)` each frame?
   What `dt`? When in the DrawPhase cycle (per DR-047) is it legal?
3. **A registry for named presets.** GPUI exposes `Animation::ease_in_out()`
   / `.spring_smooth()` / etc. as discoverable presets — sigil needs the
   same shape so `Styled::transition(TransitionId)` resolves to a
   concrete spring/tween config.
4. **A consumer for the DR-049 transition atoms.** `Style` already
   accumulates `StyleAtom::Transition(TransitionId)` and
   `StyleAtom::TransitionDuration(MillisU16)` — DR-049 §9 deferred the
   actual animation engine. This DR locks that consumer.
5. **A reduced-motion gate.** WCAG 2.2 §2.3.3 / `prefers-reduced-motion`
   media query / AccessKit `accessibility_features.reduced_motion`
   require the animator to honour user preferences. The hook must be
   uniform across all three platforms.

GPUI's shape: `Animation::new(Duration::from_millis(200)).with_easing(easing::quad_out)`
on the consumer side, and the framework ticks animations during paint
when the consumed `cx.has_active_animation(window)` is true. Sigil's
shape mirrors this almost exactly except the per-frame tick lives inside
`Runtime::begin_paint` (DR-047) so animations are subject to the same
phase invariants as everything else.

---

## Scope

### In scope (phase 1)

- `AnimationRegistry` — central store of named presets keyed by
  `TransitionId`. Ships with the same five `SpringPreset` shapes
  already in `sigil-animation` (`Gentle`, `Smooth`, `Snappy`, `Bouncy`,
  `Stiff`) plus three duration tweens (`fast = 120ms`, `default = 200ms`,
  `slow = 320ms` under `Easing::CubicInOut`).
- `Animator<T>` — owned wrapper around a `Spring` (`T = f32`) or `Tween`
  (`T = f32`) that drives a `Signal<f32>`. Public methods: `Animator::spring`,
  `Animator::tween`, `Animator::target`, `Animator::is_settled`, `Animator::value`.
- Runtime integration — animators register with `Runtime` at construction
  and unregister on drop. `Runtime::begin_paint` advances every active
  animator by the frame's `dt`, writes the new value into the bound
  `Signal<f32>`, and removes settled animators from the active set.
- `StyleAtom::Transition` consumer — when a widget's style binding
  contains `Transition(TransitionId)` and a state-slot overlay changes
  the *value* of a numeric atom (opacity, padding, border-radius, etc.),
  the engine wraps the value change in an `Animator<f32>` instead of
  applying the value directly. The signal feeding the atom becomes the
  animator's driven signal.
- Reduced-motion gate — global `AccessibilityPrefs` signal (read via
  `Cx::accessibility()` from DR-050 phase-2 surface, or a transient
  `false` fallback for phase-1 testing). When `reduced_motion = true`,
  every animator instant-settles on creation (sets value to target,
  marks settled, no per-frame ticks).
- Test seam — `RecordingAnimator` test impl that captures every
  registered animator's `(target, easing, settled_at)` for golden
  assertion.

### Out of scope (phase 2 — separate DR amendment or DR-054)

- Vector-valued springs (`Vec2`, `Vec3`, `Rgba8` colour interpolation).
  Phase 1 only ships `f32` animators; colour transitions degrade to
  instant snap until phase 2.
- `Sequence<T>` — choreographed timelines (animation 1 → animation 2 →
  animation 3 with per-step delays).
- Path-based animation (along an SVG path, Bezier, etc.).
- Physics-driven scroll (overscroll bounce, rubber-banding).
- GPU-driven animation (compute-shader particle systems).

The phase-1 surface intentionally mirrors GPUI 0.x's animation surface
which itself is f32-only with explicit colour interpolation done at the
binding-resolve layer, not the animator layer.

---

## Decisions

### Decision 1: Animator owns a `Signal<f32>`, not the reverse

The animator pushes values into a signal each frame; consumers (widgets,
style bindings) read the signal as they read any other reactive value.
This inverts the GPUI model slightly — GPUI's `Animation` is a value
the widget polls — but matches sigil's existing reactive seam where
signals are the universal subscription primitive.

```rust
let opacity = signal(0.0_f32);
let animator = Animator::spring(opacity.clone(), SpringPreset::Smooth)
    .target(1.0);  // fade-in
// `opacity` now ticks toward 1.0 each frame; binding reads `opacity.get()`.
```

The animator does not need to be retained by the caller after target is
set — it self-registers with `Runtime` on construction and self-evicts
when settled. To pre-empt and re-target a still-running animator, the
caller retains it and calls `animator.target(new_value)`.

**Why:** Two reasons.
- It composes with everything sigil already ships. `Signal<f32>` is the
  input type of every reactive widget binding; making the animator a
  signal source means *no widget code changes* to support animation —
  whatever existing widget reads `opacity` simply now reads an animated
  value.
- It survives mid-flight retargeting cleanly. GPUI's poll-based
  `Animation` requires the widget to know "an animation is in progress"
  and synchronise the swap; the signal-driven shape lets the animator
  retarget without coordination.

### Decision 2: Animators tick inside `Runtime::begin_paint`, not Effect

DR-047 §3 distinguishes Effect (state writes), Layout (read-only against
state), Paint (read-only against layout). Animator ticks write `Signal<f32>`
state — that's an Effect-shaped operation. Naively the animator should
tick during Effect.

But that creates a chicken-and-egg: animator ticks need `dt` (the frame
delta), which is only known once the frame is *starting*, which is when
`begin_paint` is called. Effect happens before Paint, so by the time
Effect runs the previous frame's `dt` is the only available value, and
the animator would always be one frame behind the layout it's
animating.

**Decision:** Animator ticks live inside `Runtime::begin_paint`. The
`pending_completions` queue established in DR-048 §4 is the canonical
sibling pattern — a phase-specific "drain at boundary" hook. Animator
writes go into a deferred queue that `end_frame` drains into the next
`begin_effect`. This is identical in shape to how `Context::spawn`
completion writes are deferred per DR-048.

**Why:**
- Frame-coherent `dt`. The animator sees the *current* frame's `dt` and
  writes the corresponding next-value, which the next `begin_effect`
  picks up; this is a one-frame delay between input and visible effect,
  which is acceptable and matches GPUI's behaviour.
- Phase invariant preserved. The Paint phase remains read-only against
  Effect-phase state — the writes are queued, not applied mid-paint.
- Reuses the DR-048 deferred-write infrastructure. No new phase
  machinery; just a new queue siblings to `pending_completions` and
  `deferred_writes`.

### Decision 3: Settled animators self-evict; tick is O(active), not O(total)

Each frame, the Runtime iterates only the *currently active* animators.
When an animator's `Spring::is_settled` returns true (or its `Tween`
reaches `t == 1.0`), the Runtime removes it from the active set. The
animator's `Signal<f32>` is left at the target value — no further writes.

To re-arm a settled animator the caller calls `animator.target(new_value)`,
which re-inserts into the active set and resumes ticking.

**Why:**
- Idle-frame cost is zero. A page with no in-flight animations costs
  nothing per frame; the iterate-active loop visits zero entries.
- Matches `cx.has_active_animation(window)` semantics in GPUI — the
  framework knows whether to schedule a re-paint by checking the active
  set size.

**Trade-off:** retained animators that are settled but kept alive (in
case of re-target) cost one `Box<dyn Tick>` retained allocation each.
This is acceptable — UI animation density is bounded (≤ 100 active per
frame in practice; widget-catalog visual goldens average <10).

### Decision 4: `AnimationRegistry` is a flat `phf` map keyed by `TransitionId`

```rust
pub struct AnimationRegistry {
    entries: phf::Map<u16, AnimationConfig>,
}
```

`TransitionId` is already `pub struct TransitionId(pub u16)` per
`crates/style/src/atom.rs:119`. The registry's lookup is `O(1)` and
its construction is `const`-eligible via `phf::phf_map!`, which lets the
default registry live as a `pub const DEFAULT: AnimationRegistry` with
zero runtime construction.

```rust
pub const DEFAULT: AnimationRegistry = AnimationRegistry {
    entries: phf::phf_map! {
        0u16 => AnimationConfig::Spring(SpringPreset::Smooth),    // sigil.transition.smooth
        1u16 => AnimationConfig::Spring(SpringPreset::Gentle),    // sigil.transition.gentle
        2u16 => AnimationConfig::Spring(SpringPreset::Snappy),    // sigil.transition.snappy
        3u16 => AnimationConfig::Spring(SpringPreset::Bouncy),    // sigil.transition.bouncy
        4u16 => AnimationConfig::Spring(SpringPreset::Stiff),     // sigil.transition.stiff
        16u16 => AnimationConfig::Tween { ms: 120, easing: Easing::CubicInOut }, // .fast
        17u16 => AnimationConfig::Tween { ms: 200, easing: Easing::CubicInOut }, // .default
        18u16 => AnimationConfig::Tween { ms: 320, easing: Easing::CubicInOut }, // .slow
    },
};
```

Applications can override the registry at startup via
`Application::animation_registry(custom)`. The override is read-only
after startup — no mid-frame swap.

**Why phf:**
- Same pattern as `sigil-icons` (already uses phf for the glyph table
  per existing crate). Familiar idiom.
- `const`-eligible map means zero per-instance cost.
- Compile-time lookup uniformity — the same registry shape works for
  test harnesses and production identically.

### Decision 5: `StyleAtom::Transition` consumes the registry at binding-install time, not paint time

When `StyleEngine` resolves a `Style` into a `StyleBinding` (existing
seam from DR-049 §9), and the binding sees `StyleAtom::Transition(id)`
*alongside* a state-slot overlay (`hover_atoms` / `focus_atoms` / etc.)
that changes a numeric atom, the engine:

1. Looks up `id` in the registry → `AnimationConfig`.
2. Reads `StyleAtom::TransitionDuration(ms)` if present (overrides the
   registry's default ms for tween configs; ignored for spring configs).
3. Constructs an `Animator<f32>` driving the binding's numeric signal
   for the affected atom (opacity / radius / padding / etc.).
4. On state-slot transition (hover entered → animator targets hover
   value; hover exited → animator targets base value), calls
   `animator.target(new)`.

**Why install-time, not paint-time:** the binding is what knows which
atoms are numeric and which have state-slot deltas; threading that
information into the paint phase would require duplicate metadata.
Binding-install runs in the Effect phase per DR-047 §5, which is the
correct phase for state setup.

**Failure mode:** `TransitionId` not in registry → log a warning via
`merge_warning_count` (existing diagnostic per `crates/style/src/merge.rs:42`)
and apply the atom value instantly (no animation). Production never sees
this if the registry is the default; custom registries that drop a
preset trip the warning at first install.

### Decision 6: Reduced-motion is a single boolean signal at the `Cx` level

```rust
impl Cx {
    /// True when the platform reports `prefers-reduced-motion`. Reads
    /// the AccessKit accessibility-features signal; falls back to false
    /// when the bridge is not yet installed (DR-050 phase 1 has no
    /// reactive bridge).
    pub fn prefers_reduced_motion(&self) -> bool;
}
```

When `prefers_reduced_motion()` returns true at animator construction,
the animator constructor:

1. Writes `target` into the bound signal instantly.
2. Marks the animator settled.
3. Does **not** register with `Runtime`.

When the preference toggles mid-session (the AccessKit reactive bridge
in DR-050 phase 2 will surface this), already-constructed animators
remain in their current state — the rule applies only at *construction*.
This is acceptable because reduced-motion is a startup-time preference
in practice on every platform.

**Why a single boolean:** WCAG 2.2 §2.3.3 is binary. Every platform we
target (Windows / macOS / Linux per the DR-040 platform matrix) exposes
the preference as a binary value through AccessKit. Sub-categorising
(e.g. "allow position animations but block opacity") is over-engineering
for the phase-1 surface and would mismatch the platform vocabulary.

### Decision 7: Test seam is `RecordingAnimator`, parallel to `RecordingBridge`

Per DR-050 §7, sigil-a11y ships a `RecordingBridge` that captures every
`publish` for assertion. The animation tests use the same pattern:

```rust
pub struct RecordingAnimator {
    pub registered: Vec<AnimatorSpec>,
    pub ticks: Vec<(TickId, f32)>,  // (animator id, value written)
}
```

Tests construct an animator, advance `Runtime::begin_paint` by N
frames at a known `dt`, and assert against the captured tick sequence.
Spring physics is deterministic at a fixed `dt`, so the goldens are
stable across runs.

A second test seam covers reduced-motion: constructing an animator
under `cx.prefers_reduced_motion() == true` and asserting (a) the
signal jumped to target instantly, (b) zero ticks were captured.

**Why this seam:** identical shape to the a11y bridge seam means the
mental model transfers (one of the W4 substrate consistency wins —
the same testing pattern across G7-G11). The `Recording*` shape is
also the prior art in the audio engine's `RecordingMixer` from the
`the host engine` audio crate.

---

## Public API surface

The animator surface lives in a new `sigil-animation-runtime` crate
(separate from `sigil-animation` to keep the pure-math primitives
deployable to wasm without the Runtime / Signal dependencies).

```rust
// crates/animation_runtime/src/lib.rs

use sigil_animation::{Easing, Spring, SpringPreset, Tween};
use sigil_reactive::{Runtime, Signal};
use sigil_style::TransitionId;

/// Animator config from the registry — either a spring shape or a
/// duration tween. Re-targetable after construction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnimationConfig {
    /// Hooke's-law spring with a named preset.
    Spring(SpringPreset),
    /// Fixed-duration tween under an easing curve.
    Tween { ms: u16, easing: Easing },
}

/// `phf`-backed registry of named animation presets. Construct via
/// [`AnimationRegistry::default`] or override per-app at startup.
pub struct AnimationRegistry {
    entries: phf::Map<u16, AnimationConfig>,
}

impl AnimationRegistry {
    /// Look up a config by `TransitionId`.
    pub fn get(&self, id: TransitionId) -> Option<AnimationConfig>;
}

/// Phase-1 animator. Drives a `Signal<f32>` toward a target value.
pub struct Animator {
    // Implementation detail — opaque to consumers.
    inner: Box<dyn AnimatorTick>,
}

impl Animator {
    /// Construct a spring-shaped animator driving `signal`. Initial
    /// position = `signal.peek()`. Caller calls `target` to set the
    /// destination; the animator self-registers with the active
    /// `Runtime`.
    #[must_use]
    pub fn spring(signal: Signal<f32>, preset: SpringPreset) -> Self;

    /// Construct a tween-shaped animator. Same self-registration
    /// behaviour. `ms` is the duration in milliseconds.
    #[must_use]
    pub fn tween(signal: Signal<f32>, ms: u16, easing: Easing) -> Self;

    /// Set the destination value. Re-arms a settled animator.
    pub fn target(&mut self, value: f32);

    /// Peek the current value (untracked — does not register a
    /// subscription).
    pub fn value(&self) -> f32;

    /// True when the underlying spring/tween reports settled.
    pub fn is_settled(&self) -> bool;
}

impl Drop for Animator {
    fn drop(&mut self) {
        // Auto-deregister from Runtime when the owner drops.
    }
}

/// Per-frame tick contract. Implementations exist for `Spring` (via
/// `SpringAnimator`) and `Tween` (via `TweenAnimator`); the registry
/// constructs the right one from `AnimationConfig`.
trait AnimatorTick: Send {
    fn tick(&mut self, dt: f32) -> AnimatorStatus;
    fn target(&mut self, value: f32);
    fn value(&self) -> f32;
    fn is_settled(&self) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AnimatorStatus {
    /// Animator is still moving; keep ticking next frame.
    Running,
    /// Animator settled this frame; Runtime drops it from active set.
    Settled,
}

/// Test seam — captures every animator registered with the runtime
/// and every tick written. See DR-053 §7 and `RecordingBridge` parallel
/// in `crates/a11y/src/bridge.rs:23`.
#[cfg(any(test, feature = "test-recording"))]
pub struct RecordingAnimator {
    pub registered: Vec<RegistrationSpec>,
    pub ticks: Vec<TickRecord>,
}
```

### Runtime integration

```rust
// In crates/reactive/src/runtime_animator.rs (new module):

impl Runtime {
    /// Phase-1 animator tick. Called by `begin_paint` per DR-047 §3.
    /// Drains the active-animator set; writes go into the deferred
    /// queue and apply at the next `begin_effect`.
    pub(crate) fn tick_animators(&mut self, dt_seconds: f32);

    /// Register an animator with this Runtime. Called by
    /// `Animator::spring` / `Animator::tween` constructors.
    pub(crate) fn register_animator(&mut self, animator: Box<dyn AnimatorTick>) -> AnimatorId;

    /// Deregister. Called by `Animator::drop`.
    pub(crate) fn unregister_animator(&mut self, id: AnimatorId);
}
```

### Cx surface

```rust
impl Cx {
    /// True when the user prefers reduced motion. Phase 1: returns
    /// false until DR-050 phase 2 lands the AccessKit reactive bridge.
    /// Phase 2: reads `accessibility_features::REDUCED_MOTION` from the
    /// installed bridge.
    pub fn prefers_reduced_motion(&self) -> bool;
}
```

### `Styled::transition` consumer wiring

The existing `Styled::transition(TransitionId)` / `Styled::transition_duration`
methods (DR-049) require no API change. The binding resolver in
`crates/style/src/binding.rs` gains a new hook: when assembling a
`StyleBinding` it inspects the resolved atom list for a `Transition`
atom; if present and a state-slot overlay differs in a numeric atom,
the resolver constructs an `Animator<f32>` driving the binding's
existing numeric signal for the affected atom.

```rust
// Pseudo-code for the binding-install hook:
fn install_animations(binding: &mut StyleBinding, registry: &AnimationRegistry) {
    let Some(id) = binding.transition_id() else { return };
    let Some(config) = registry.get(id) else {
        merge_warning_count_for_component(ComponentId::AnimationRegistryMiss).bump();
        return;
    };
    for slot in [Slot::Base, Slot::Hover, Slot::Focus, Slot::Pressed, Slot::Disabled] {
        let Some(numeric_signal) = binding.numeric_signal_mut(slot, NumericAtom::Opacity) else { continue };
        // Drive the numeric signal via an Animator; binding retains the
        // animator so its drop is tied to binding eviction.
        let animator = match config {
            AnimationConfig::Spring(preset) => Animator::spring(numeric_signal.clone(), preset),
            AnimationConfig::Tween { ms, easing } => {
                let ms = binding.transition_duration_ms().unwrap_or(ms);
                Animator::tween(numeric_signal.clone(), ms, easing)
            }
        };
        binding.attach_animator(slot, NumericAtom::Opacity, animator);
    }
}
```

---

## Implementation phases

### Phase 1 (this DR)

1. New crate `crates/animation_runtime/` with `Animator`, `AnimationRegistry`,
   `AnimationConfig`, `RecordingAnimator`.
2. `Runtime::tick_animators` + `register_animator` + `unregister_animator`
   plus per-frame integration in `begin_paint`.
3. `Cx::prefers_reduced_motion` returning false (placeholder; real
   reading lands in DR-050 phase 2).
4. Binding-install hook in `sigil-style` consuming `StyleAtom::Transition`
   for `f32` atoms (opacity, padding, border-radius, line-height,
   font-size, gap).
5. Tests:
   - `spring_animator_settles_at_known_frame_count`
   - `tween_animator_at_t_eq_one_equals_target`
   - `retarget_during_animation_continues_from_current_value`
   - `dropping_animator_deregisters_from_runtime`
   - `reduced_motion_skips_animator_registration`
   - `unknown_transition_id_logs_merge_warning`
   - `binding_install_attaches_animator_for_hover_opacity_delta`

### Phase 2 (separate DR)

- Vector-valued animators (`Vec2`, `Rgba8`, layout `Dimension`).
- `Sequence<T>` for choreographed timelines.
- Per-property reduced-motion exemptions (essential motion like loading
  spinners).
- GPU-driven animation for high-density particle effects.

---

## Tests

Phase 1 ships seven tests as enumerated above plus one golden test
asserting `visual_goldens/button_hover.png` is byte-identical to the
DR-049 baseline at frame 0 *and* visibly different from frame 0 at
frame N=8 under `SpringPreset::Smooth` (~130ms settle). This guards the
binding-install hook against a regression where the animator never
attaches.

Golden frame N is captured at fixed `dt = 1.0 / 60.0` so spring physics
is deterministic across CI runs.

---

## Open questions

- **Per-binding vs per-atom animator instances.** Phase 1 ships one
  animator per (binding, slot, numeric_atom) triple — six atoms × four
  slots × N bindings = up to 24×N animators per frame in the worst
  case. If the visual goldens average 5 widgets with 1 active state
  delta each, that's ~10 active animators per frame, well within
  budget. If practice shows higher density, phase 2 can collapse to
  one animator per (binding, slot) with a tuple value type.

- **Spring vs tween default for the unspecified case.** When `Styled::transition`
  is called without `transition_duration`, the default registry entry
  for the most common preset (`Smooth`) is a spring, not a tween — so
  the duration arg is ignored. This may surprise consumers who pass a
  duration expecting tween-style behaviour. The chosen behaviour matches
  GPUI; flagged for downstream-developer docs (`dogfood-newdev` review).

- **Animator pooling.** Each `Animator` allocates a `Box<dyn AnimatorTick>`.
  At ~24 simultaneous animators per heavily-animated screen the per-frame
  allocator churn is bounded (each animator allocates once at
  registration, frees once at deregistration), but a pool could amortise
  this if profiling shows it as a hot path. Deferred until measurement
  motivates it.

---

## References

- DR-040 §4 — GPUI parity thesis lock (animation is one of six pillars).
- DR-047 §3 — DrawPhase Effect/Layout/Paint contract.
- DR-048 §4 — deferred-write queue pattern that this DR mirrors for
  animator writes.
- DR-049 §9 — `Styled::transition` + `transition_duration` atom emission
  with consumer deferred to this DR.
- DR-050 §6 — accessibility live-region surface; phase 2 will surface
  `prefers_reduced_motion`.
- `crates/animation/src/spring.rs:1` — existing `Spring` semi-implicit
  Euler integration (the math this DR layers a runtime over).
- `crates/animation/src/easing.rs:1` — existing `Easing` curve set.
- `crates/animation/src/tween.rs:1` — existing `Tween` fixed-duration
  primitive.
- `crates/a11y/src/bridge.rs:23` — `RecordingBridge` prior art for the
  `RecordingAnimator` test seam.
- `crates/style/src/atom.rs:117-123` — existing `TransitionId(u16)` +
  `MillisU16(u16)` atom payloads consumed here.
- `docs/plans/2026-05-16-gpui-parity-integration.md:108` — parity-matrix
  row for G9.
- `crates/animation/src/spring.rs:29-44` — `SpringPreset` enum (Gentle /
  Smooth / Snappy / Bouncy / Stiff) reused as the default registry
  configs.
- W3C WCAG 2.2 §2.3.3 *Animation from Interactions* — reduced-motion
  conformance ground truth.
