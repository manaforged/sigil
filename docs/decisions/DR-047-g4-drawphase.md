---
id: DR-047
status: proposed
date: 2026-05-18
covers: G4 DrawPhase state machine + write-during-Paint assertion seam (W3.2 design, W3.2-impl implementation)
depends_on: [DR-040, DR-045, DR-046, DR-034]
relates: docs/plans/2026-05-18-parallel-dispatch-master.md, docs/specs/g4-drawphase-api-sketch.md
builds_on: a4b022a
---

# DR-047 — G4 DrawPhase State Machine + Write-During-Paint Assertion Seam

**Status:** Proposed
**Date:** 2026-05-18
**Covers:** G4 keystone — `DrawPhase` enum on `RuntimeState`, the four
phase transitions (Effect → Layout → Paint → Idle → Effect), the
debug-assert seam on every reactive write path, the deferred-write
queue that swallows Paint-phase action dispatches and replays them in
the next Effect phase.
**Depends on:** DR-040 (sigil thesis lock — mirror GPUI's validated
substrate), DR-045 (`Entity<T>` / `Context<'_, T>` — the write seam G4
asserts on), DR-046 (typed action dispatch — the action-write seam),
DR-034 (layer pool + two-phase frame walker — the Paint pass G4
brackets).
**Unlocks:** G5 `Task<T>` (task completion handlers run in Effect,
never Paint — DR-048 will cite this DR), Phase F (sigil-events
writes-during-input-handling guards), the visual-gate-essential
reproducibility argument (a frame can no longer self-invalidate
mid-Paint).

---

## Context

GPUI partitions every window frame into a small fixed set of phases
and enforces by panic that certain operations are only legal in
certain phases. The headline phase is **Paint**, which is read-only
with respect to the reactive graph: a widget's `paint` method may
read entity state, layout output, glyph metrics, image bytes — and
may write to the encoder, the layer pool, the GPU command buffer —
but it must not mutate reactive state. Any `entity.update`,
`signal.set`, `cx.dispatch_action`, or `cx.notify` that lands during
Paint trips a debug panic at the seam.

The reason this invariant is load-bearing has two halves:

1. **A write during Paint invalidates the very frame being painted.**
   `entity.update` schedules observers; observers re-run effects;
   effects may dirty layout; dirty layout invalidates the paint tree
   the renderer is currently walking. The renderer either ignores
   the invalidation (and paints torn state) or honours it (and loops
   forever, since each frame self-invalidates). Both outcomes are
   bug-class. The clean fix is structural: forbid the write, panic
   on debug, defer-or-drop on release.
2. **Without the invariant, the frame graph cannot be reasoned
   about.** Layout-then-Paint-then-Composite is a pipeline; every
   stage assumes the previous stage's output is frozen. A widget
   that mutates an entity from inside its paint code breaks that
   freeze for whatever observers happen to chain off the entity.
   Two-phase frame walking (DR-034) already assumes a frozen scene
   command buffer; G4 generalises that assumption from "the command
   buffer is frozen" to "the entire reactive graph is frozen for the
   duration of Paint."

What currently exists in sigil:

- **`crates/reactive/`** ships the full substrate G4 will guard:
  `Signal<T>::set`/`update` (DR-005), `Entity<T>::update` /
  `Context::notify` / `Context::emit` (DR-045), `Context::on_action`
  / `Context::dispatch_action` (DR-046, landed on `a4b022a`).
  Every one of those paths takes a `with_rt(|rt| ...)` mutable borrow
  on `RuntimeState`, which is the natural place to consult a phase
  field.
- **`crates/renderer/`** owns the two-phase frame walker (DR-034):
  `build_tree` (flat scene → `Vec<Node>`) then `render_nodes`
  (depth-first walk emitting GPU commands). The walker is the
  authoritative "we are currently painting" region.
- **`crates/layout/`** owns the owned `LayoutDrive` (DR-041). Layout
  is invoked once per frame from the frame driver.
- **No phase enforcement exists.** Today, calling `entity.update`
  from inside a renderer hook compiles, runs, and silently corrupts
  the frame. The renderer doesn't know it has been re-entered; the
  reactive runtime doesn't know it has been called from Paint.

What does not exist:

- **No `DrawPhase` enum.** No central record of "what is the runtime
  doing right now?"
- **No transition seams.** No `begin_paint` / `end_paint` calls
  bracketing the renderer's two-phase walk.
- **No write-path assertions.** `Signal::set`, `Entity::update`,
  `Context::notify`, `Context::dispatch_action` all proceed in any
  phase.
- **No deferred-write queue.** A Paint-phase action dispatch from a
  release build (where the debug-assert is compiled out) is silently
  ignored today; the GPUI shape is to queue the dispatch for the
  next Effect phase.

The mistake to avoid is hacking a `painting: bool` flag onto the
renderer and reading it from one or two write paths. That produces a
half-enforced invariant: future writes added to the runtime
(`cx.emit`, `cx.set_global`, future G5 `Task::schedule`) will not
know to consult the flag. The correct shape is a phase field on the
runtime, set by the frame driver, consulted by every write path
through one helper. One enum, one field, one helper, every write
guarded the same way.

### Reading the existing repo

- `crates/reactive/src/runtime.rs` lines 192–243 — `RuntimeState`
  layout. G4 adds one field (`phase: DrawPhase`) here.
- `crates/reactive/src/runtime.rs` lines 246–267 — `RuntimeState::new`
  `const fn`. G4 must keep this `const`; `DrawPhase::Effect` is the
  initial value and is a valid `const`.
- `crates/reactive/src/cx.rs` lines 66–80 — `Context::notify`,
  representative write seam. G4 inserts one `assert_paint_safe`
  call at the top of `with_rt`-protected writes.
- `crates/reactive/src/signal_write.rs` lines 98–135 — `Signal::set`,
  the deepest hot path. G4's assertion runs inside `with_rt` before
  the slot mutation. Same shape for `Signal::update`,
  `Entity::update::run_update`, `Context::notify`, `Context::emit`,
  `Context::dispatch_action`, `Context::set_global`.
- `crates/reactive/src/entity.rs` lines 234–288 —
  `run_update` already takes the runtime borrow once at entry and
  once at exit. The phase assert goes at entry, before the
  `slot.locked` check.
- DR-034 (layer pool + two-phase walker) — the renderer hook G4
  brackets. The frame driver calls `Runtime::begin_paint()` before
  `build_tree`, and `Runtime::end_paint()` after `render_nodes`
  returns. The walker itself is unmodified.

---

## Decision

Eight decisions, each frozen for the W3.2-impl dispatch.

### 1. Phase enum — `Effect | Layout | Paint | Idle`, stored as a `Cell`-free field on `RuntimeState`

```rust
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum DrawPhase {
    Effect,
    Layout,
    Paint,
    Idle,
}
```

Four variants, in canonical frame order. `Effect` is the reactive
fan-out phase — observers run, signals fire, action handlers
dispatch. `Layout` is when `LayoutDrive::compute` walks the styled
node tree. `Paint` is the renderer's two-phase walk from DR-034
(build_tree + render_nodes). `Idle` is the gap between frames — no
phase is active, the runtime is dormant. The default value at
runtime startup (and between frames) is `Idle`.

The phase lives on `RuntimeState` as a plain field:

```rust
pub(crate) phase: DrawPhase,
```

**Not a `Cell<DrawPhase>`.** Every read and write of the phase goes
through `with_rt(|rt| ...)`, which already takes an exclusive
mutable borrow on `RuntimeState`. A `Cell` would add interior
mutability where there is none needed and would invite reads outside
the `with_rt` borrow boundary — which is exactly the access discipline
the rest of the crate avoids. One field, one borrow path, no extra
synchronisation primitives.

`DrawPhase: Copy` and the `RuntimeState::new` initialiser
(`Self { ..., phase: DrawPhase::Idle }`) stay `const`-constructible
for the thread-local `const` block (per `runtime.rs` line 271).
`Copy` also means `rt.phase` reads do not move the value, and the
debug-assert helper can capture it for the panic message without
borrow gymnastics.

### 2. Single assertion seam — `assert_paint_safe(rt, site)` helper on the runtime

Every write path consults the phase through one helper:

```rust
#[track_caller]
pub(crate) fn assert_paint_safe(rt: &RuntimeState, site: &'static str) {
    debug_assert_ne!(
        rt.phase,
        DrawPhase::Paint,
        "{site} called during DrawPhase::Paint — reactive writes are illegal in the paint pass \
         (call site: {})",
        Location::caller(),
    );
}
```

Call sites:

- `Signal::set` — `assert_paint_safe(rt, "Signal::set")` inside the
  `with_rt` block, before the slot mutation.
- `Signal::update` — same, label `"Signal::update"`.
- `Entity::update` (through `run_update`) — same, label
  `"Entity::update"`. Fires before the `slot.locked` check so a
  Paint-phase update gives a phase-specific panic, not a confusing
  "already locked" message.
- `Context::notify` — same, label `"Context::notify"`.
- `Context::emit` — same, label `"Context::emit"`.
- `Context::dispatch_action` — same, label
  `"Context::dispatch_action"`.
- `Context::set_global` — same, label `"Context::set_global"`.
- `cx.focus(handle)` / `cx.blur(handle)` — these mutate
  `focus_stack`, which retargets future action dispatch; treat as
  writes. Label `"Context::focus"` / `"Context::blur"`.

Read paths (`Signal::get`, `Signal::peek`, `Entity::read`,
`Context::is_focused`, `cx.global::<G>`) are **not** guarded.
Reading from Paint is the entire point of Paint — widget paint code
reads entity state, signal values, layout output, focus state. Only
mutations are guarded.

`debug_assert_ne!` (not `assert_ne!`) means the check compiles out
of release builds. The release-build behaviour for the same illegal
write is decision 5 (deferred-write queue, no panic).

**Why a free function on the runtime module, not a method on
`Context` or `RuntimeState`:** the write seams are spread across
seven files (`signal_write.rs`, `entity.rs`, `cx.rs`,
`entity_observer.rs`, `entity_global.rs`, ...). A free function in
`runtime.rs` is `pub(crate)`-visible to all of them through the
existing module graph. A method on `Context<'_, T>` would not be
callable from inside `with_rt` (Context borrows are user-facing,
not runtime-facing). A method on `RuntimeState` would work but adds
a layer of indirection for no win.

### 3. Phase transitions — owned by the frame driver, public API on `Runtime`

Phase transitions happen in exactly one place: the frame driver, an
external caller (the renderer, or in tests the harness) calling
public methods on `Runtime`:

```rust
impl Runtime {
    pub fn begin_effect();   // Idle → Effect
    pub fn begin_layout();   // Effect → Layout
    pub fn begin_paint();    // Layout → Paint
    pub fn end_frame();      // Paint → Idle (replays deferred writes)
}
```

Calling these in the wrong order panics in debug:
`begin_layout` while phase != Effect, etc. In release the transition
proceeds anyway — getting the phase machine wrong is a frame-driver
bug, not a user bug, and the renderer is in-tree so we control the
invariant.

The frame driver wraps one frame as:

```rust
Runtime::begin_effect();
// observers drained, action handlers run, effects fire
Runtime::begin_layout();
// LayoutDrive::compute() walks the styled tree
Runtime::begin_paint();
// renderer build_tree + render_nodes (DR-034)
Runtime::end_frame();
// replays deferred-write queue → next frame's Effect phase
```

`Idle` is the resting phase. Tests that do not drive a frame run
entirely in `Idle`, and `Idle` allows writes (it is not Paint). Most
existing reactive tests touch the runtime in `Idle` without knowing
phase exists — G4 preserves this behaviour by making `Idle` the
default.

**Why expose four discrete `begin_*` / `end_frame` calls and not a
single `Runtime::frame(|cx| ...)` scope:** the four phases are not
nested. They are sequential pieces of one frame, each potentially
spanning multiple `with_rt` borrow boundaries, each owned by a
different crate (Effect = reactive; Layout = sigil-layout; Paint =
renderer). A single scope closure would force every consumer of
every phase into one crate's call site. Discrete entry points let
the renderer crate (which does not depend on reactive's effect
machinery) own its `begin_paint`/`end_frame` calls and let the
sigil-layout crate own its `begin_layout`. Each crate transitions
when it is ready; the frame driver is the orchestrator.

### 4. Phase observability — `Runtime::current_phase()` is public; `Context::is_painting()` is the ergonomic shortcut

```rust
impl Runtime {
    pub fn current_phase() -> DrawPhase;
}

impl<T: 'static> Context<'_, T> {
    pub fn is_painting(&self) -> bool;  // shorthand
}
```

`current_phase()` reads `rt.phase` through `with_rt`. Calls are
cheap (`Copy` enum, one thread-local borrow, no allocation).

The public-API rationale: a widget's `paint` method may legitimately
want to know "am I currently painting?" — for example, to choose
between a cheap path (write to layer-pool texture) and the
write-deferred path (queue a state change for next frame). Exposing
the phase as readable lets authors write that branch explicitly.

We do not expose phase setters publicly. The `begin_*` / `end_frame`
methods on `Runtime` are public because the frame driver is part of
the consuming application; in sigil's case the frame driver lives in
`crates/sigil` (the top crate), so it imports `Runtime::*` like any
other consumer. Third-party consumers (a custom embedder) may call
the same entry points; that is the seam the embedder needs.

Inside the assert panic message, the phase appears as part of the
formatted string ("called during DrawPhase::Paint"). Consumers
debugging an assert do not need a separate phase API to investigate;
the panic itself carries the phase.

### 5. Release-build behaviour — deferred-write queue, drained at `end_frame`

`debug_assert_ne!` compiles out of release. In release the runtime
must still do something safe with a Paint-phase write — silently
proceeding corrupts the frame; outright dropping the write loses the
user's intent. The third option, **defer**, matches GPUI:

```rust
pub(crate) struct DeferredWrite {
    kind: DeferredKind,
    caller: &'static Location<'static>,
}

pub(crate) enum DeferredKind {
    Action(Box<dyn Action>),
    // future: spawn, signal_set, etc. — see Open questions.
}
```

A `RuntimeState::deferred_writes: Vec<DeferredWrite>` holds the queue.
**Only `dispatch_action` defers in G4.** Action dispatch is the
common Paint-phase write — a click handler attached to a widget that
runs during input dispatch (Effect phase) is fine, but the GPUI
pattern of "fire-and-forget dispatch from inside Paint" (typically
analytics or telemetry) needs to work. Action dispatches captured
during Paint go into the queue. `end_frame` replays them at the top
of the next Effect phase.

`signal.set` / `entity.update` / `cx.notify` / `cx.emit` /
`cx.set_global` / `cx.focus` / `cx.blur` **do not defer.** They
panic in debug and are no-ops in release. The reasoning is
asymmetric:

- Direct state mutations from inside Paint are almost always bugs
  — a widget computing in paint should not be the authority that
  changes the state it is reading. Deferring would mask the bug
  while still producing wrong rendering this frame.
- Action dispatch is the legitimate Paint-phase escape hatch.
  Actions are the indirection layer between "a thing happened" and
  "state mutates"; the queue is the natural place to record that
  thing-happened signal until the next safe write window. A
  Paint-phase `dispatch_action(&AnalyticsTick { ... })` is fine.

The queue is bounded by reasonable Paint-phase dispatch frequency
(< 10 per frame in any sane consumer). At `end_frame` the queue is
drained in arrival order: each `DeferredWrite::Action` is replayed
through `Context::dispatch_action` against the **focus stack as it
exists at replay time**, not the stack as it existed at capture
time. This matches "action dispatches are intent expressed against
the live focus, not a snapshot."

The queue does not persist across frames in the sense of carrying
unconsumed entries forward: if `end_frame` runs and the queue is
drained, the next frame starts fresh. Replay during drain may
itself enqueue new deferred writes (a replayed action handler
dispatches another action — recursively rare but possible); those
land in the queue and are drained in the same `end_frame` pass via
a fixed-iteration cap (decision 8).

### 6. G3 action-dispatch hookup — `Context::dispatch_action` is the citation point

`Context::dispatch_action` (DR-046 decision 6, file `crates/reactive/src/cx.rs` line 323) gains the phase
guard at its entry point:

```rust
pub fn dispatch_action<A: Action>(&mut self, action: &A) {
    // G4 hookup — DR-047 §6.
    let painting = with_rt(|rt| rt.phase == DrawPhase::Paint);
    if painting {
        // debug build: panic via assert_paint_safe (consistent message)
        // release build: defer the action to the next Effect phase
        defer_action_or_panic::<A>(action, Location::caller());
        return;
    }
    // ... existing dispatch implementation ...
}
```

`defer_action_or_panic` is the helper that wraps `assert_paint_safe`
+ deferred-write enqueue. In debug it asserts (panic). In release it
boxes the action and pushes onto `RuntimeState::deferred_writes`.

DR-046's "G4 DrawPhase gains an action-handler boundary to assert
on" line in its Consequences section is satisfied by this DR through
this seam. The DR-046 implementation already exists on `a4b022a`;
G4-impl will add the four lines above without modifying the
dispatch core.

`Context::on_action` (handler registration) is **not** a write seam
that asserts. Registration mutates the action registry, but the
registry is consumed by the next dispatch — registering during Paint
is harmless because the registered handler doesn't fire until a
later dispatch. Same shape for `cx.focus_handle()`: minting a handle
is registration, not a focus change.

### 7. G5 Task<T> implications — task completion handlers run in Effect, never Paint

The pending DR-048 (G5 `Task<T>` + `cx.spawn`) handles long-running
async work. A task completion handler is the code that runs when the
awaited future resolves; in GPUI this code routinely calls
`entity.update` to apply the result. G5 must guarantee the
completion handler runs in Effect, not Paint.

The seam is the task scheduler's drain point. The G5 implementer
(DR-048 W3.3-impl) must:

- Drain completed tasks at the top of `Runtime::begin_effect()`, not
  on demand. Any task whose future resolved mid-Paint is queued and
  fired at `begin_effect` — same shape as the deferred-write queue,
  but for completion callbacks instead of pending dispatches.
- Forbid `cx.spawn` from running its handler synchronously; the
  handler always lands in a later Effect phase. This eliminates the
  "task completed instantly, handler ran in Paint" race.

G4 reserves the wiring point (`RuntimeState::deferred_writes`
generalises to `DeferredKind::TaskComplete(...)` in DR-048) but does
**not** ship the task plumbing itself. The G5 DR will cite this DR
for the completion-handler phase contract.

### 8. Subscription / observer callback phase — fired during the same `update_depth == 0` drain that already exists, all of which lives in Effect

The existing pending-observer drain (`drain_pending_observers` in
`entity_observer.rs:290`) runs when the outermost `Entity::update`
exits. Observer callbacks may themselves call `entity.update`,
`signal.set`, `dispatch_action` — and those writes must succeed,
which means the drain must run in Effect.

The frame driver invariant is: **the entire reactive write graph,
including all observer fan-out, completes before `begin_layout` is
called.** This is the existing GPUI invariant ("effects flush before
layout"); G4 codifies it as a phase contract.

Implementation: at the top of `Runtime::begin_layout()`, the runtime
loops `drain_pending_observers` until the pending queue is empty
**and** the deferred-write queue is empty. A fixed iteration cap
(`MAX_DRAIN_PASSES = 16`) catches a runaway observer cycle and
panics with an actionable message; in normal use 1–3 passes is the
expected count.

Effects (signal-driven, scheduled into `RuntimeState::pending`) drain
the same way — `schedule_effects` already runs them synchronously
outside `batch`. G4 does not modify the effect-scheduling shape; it
only requires that the entire fan-out completes before phase
advances to Layout.

Paint-phase observer callbacks **cannot fire**, because by decision
2 no write can land in Paint, so no `cx.notify`/`cx.emit` can
schedule an observer. The deferred-write queue (decision 5) is the
escape valve, but it carries actions, not observer notifications —
an action that the next Effect phase dispatches will, of course,
schedule its own observers, but those run in Effect.

This composes cleanly with DR-045 §5 ("notify() propagation —
batched, drained at the end of the dispatch turn"): the existing
batched drain is what G4 brackets. G4 adds the phase machine around
the drain, not inside it.

---

## Implementation seams

- **New module:** `crates/reactive/src/phase.rs`, ~120 LoC budget.
  Owns the `DrawPhase` enum, `assert_paint_safe`,
  `defer_action_or_panic`, the deferred-write queue type, and the
  drain helper. `pub use phase::DrawPhase` from `lib.rs`.
- **`crates/reactive/src/runtime.rs`** — `RuntimeState` gains two
  fields: `phase: DrawPhase` (default `Idle`) and `deferred_writes:
  Vec<DeferredWrite>`. `RuntimeState::new` initialises both as the
  trivial defaults (still `const`-constructible).
- **`crates/reactive/src/runtime.rs`** — `Runtime` gains four
  public methods (`begin_effect`, `begin_layout`, `begin_paint`,
  `end_frame`) plus `current_phase`. ~50 LoC across the four
  transitions. Each is a `with_rt` borrow + phase update + (for
  `begin_layout`) the observer-drain loop, (for `end_frame`) the
  deferred-write drain.
- **`crates/reactive/src/signal_write.rs`** —
  `Signal::set` and `Signal::update` gain one
  `assert_paint_safe(rt, "Signal::*")` line each, inside the
  existing `with_rt` block.
- **`crates/reactive/src/entity.rs`** — `run_update` gains one
  `assert_paint_safe(rt, "Entity::update")` at the top of its entry
  `with_rt` block, before the `slot.locked` check.
- **`crates/reactive/src/cx.rs`** — `notify`, `emit`,
  `dispatch_action`, `focus`, `blur` gain one `assert_paint_safe`
  each. `dispatch_action` additionally branches into the
  deferred-write path (decision 6).
- **`crates/reactive/src/entity_global.rs`** —
  `set_global` gains `assert_paint_safe`.
- **`crates/reactive/tests/drawphase.rs`** (new) — phase contract
  test suite (test outline in the API sketch).
- **No `crates/sigil/src/draw_phase.rs`.** The master plan W3.2
  brief named that file, but the keystone is reactive-crate code,
  not sigil-crate code — the assertion seam lives at the write
  source, not the consumer. The W3.2-impl dispatch will land code
  in `crates/reactive/`, with a thin re-export in `crates/sigil/`
  if a sigil-crate consumer (the frame driver) ever needs the
  type by sigil-name rather than reactive-name.
- **No renderer changes in W3.2-impl.** DR-034's two-phase walker
  is the consumer of `begin_paint` / `end_frame`, but the
  renderer's `begin_paint`/`end_frame` wiring is a separate
  follow-up (call it W3.2b) once the reactive seam is green.

---

## Consequences

- **One field added to RuntimeState (`phase`), one field added
  (`deferred_writes`), one new module (`phase.rs` ~120 LoC).** No
  changes to the arenas, no changes to the existing drain logic, no
  changes to the effect scheduler.
- **Eight write paths gain a one-line debug-assert.** Total touched
  files: 4 (`signal_write.rs`, `entity.rs`, `cx.rs`,
  `entity_global.rs`). Each addition is a single line inside an
  existing `with_rt` block.
- **The runtime gains four public lifecycle methods on `Runtime`:**
  `begin_effect`, `begin_layout`, `begin_paint`, `end_frame`. These
  are the seams the future frame driver calls. They are also the
  seams tests use to drive the phase machine deterministically.
- **Paint-phase release-build action dispatches survive via the
  deferred-write queue.** Authors who write
  `cx.dispatch_action(&AnalyticsTick { ... })` from a widget's
  paint code see correct behaviour in both debug (panic — fix it)
  and release (defer — works). The deferral is a documented
  behaviour, not a silent fallback.
- **Tests gain `Runtime::begin_paint() ... Runtime::end_frame()`
  bracketing.** Any test that wants to verify "this write is
  rejected during Paint" wraps the write in those calls. The phase
  test suite (decision 8 / API sketch) is the canonical pattern.
- **G3 is closed.** DR-046's open question 1 (action-handler phase
  enforcement) is resolved here through decision 6.
- **G5 has its completion-handler phase contract.** DR-048 cites
  this DR for the "task completion runs in Effect" rule.
- **The DR-034 two-phase walker is no longer free to write.** This
  is not a regression — DR-034 already assumed a frozen scene
  command buffer; G4 enforces that assumption against the broader
  reactive graph. The walker itself does not call into reactive
  writes; the renderer's per-widget paint hook is the only place
  that might, and that hook is exactly the place G4's panic
  message points at.
- **No widget-API changes.** Existing widgets compile unchanged.
  Existing tests pass unchanged (they run in `Idle`, which allows
  writes). The phase machine is opt-in: a consumer not calling
  `begin_paint` never enters Paint and never trips the assert.
- **Bench surface — small.** The `assert_paint_safe` helper is
  one `assert_ne!` of two `Copy` enum values inside an already-live
  `with_rt` borrow; the compiled cost in debug is ~2 instructions
  and in release is zero. No criterion bench required for W3.2-impl.

---

## What this does NOT cover

Explicit scope cuts. These are out for W3.2 and W3.2-impl; some
have a follow-up DR scheduled, others are explicitly deferred.

- **Frame driver implementation.** Who calls `begin_effect` /
  `begin_layout` / `begin_paint` / `end_frame`, in what order, on
  what cadence, from which thread — none of this is in G4. G4
  ships the phase machine; the frame driver is W3.2b (or whichever
  later wave wires the renderer's per-frame loop).
- **Render-pass batching.** How the renderer batches draws inside
  Paint is DR-034's territory. G4 brackets Paint; it does not
  describe what happens inside.
- **Scheduler design.** There is no scheduler in G4. The phase
  machine has no notion of "priority," "starvation avoidance," or
  "frame budget." Adding any of those is a separate decision.
- **Cross-thread phase enforcement.** The reactive runtime is
  thread-local (`!Send`, single-thread). The phase is per-thread.
  Background tasks (G5) running on other threads cannot observe
  another thread's phase; they shuttle work back via the main
  thread's runtime, which has its own phase. Cross-thread phase
  semantics are not a G4 concern.
- **Deferred-write semantics for non-action writes.** G4 only
  defers `dispatch_action`. Adding deferral for `entity.update` /
  `signal.set` is a future call; today they panic in debug and
  no-op in release. The asymmetry is decision 5.
- **Replay ordering across multiple deferred writes from the same
  Paint pass.** Decision 5 specifies arrival order at the queue,
  which is identical to GPUI's behaviour. No re-ordering, no
  priority.
- **Phase enforcement on `Signal::get` / `Entity::read` / other
  read paths.** G4 guards writes only. Reads in Paint are
  legitimate.
- **The `Idle` phase as a queue-drain trigger.** `Idle` is the
  resting state, not a phase the runtime actively does work in.
  Background-task completion drains happen at `begin_effect`, not
  at the `Idle → Effect` transition (which is the same instant).
- **Custom phases / user-defined phases.** The enum is closed: four
  variants, no extension mechanism. A future need for "Animate"
  or "PostPaint" phases would require a DR amendment.

---

## Alternatives considered

### `Cell<DrawPhase>` on `RuntimeState`

Rejected. Interior mutability invites reads outside the `with_rt`
borrow, which is the access discipline the rest of the crate
avoids. Every reactive op already takes `with_rt(|rt| ...)`; a
plain `pub(crate)` field on `RuntimeState` reuses that borrow path
and stays consistent with `current_owner`, `current_entity`,
`batch_depth`, etc.

### Phase as a thread-local `Cell<DrawPhase>` outside `RuntimeState`

Rejected. Two thread-locals (RUNTIME + PHASE) doubles the borrow
surface and adds a sync question between them ("can phase change
while a `with_rt` borrow is live?"). One thread-local with one
phase field has neither problem.

### Single `Runtime::frame(|cx| ...)` scope closure

Rejected. The four phases span three crates (reactive, layout,
renderer); a single closure would force the frame driver into a
single call site, which means the orchestration logic and the
per-phase consumers cannot be cleanly separated. Discrete entry
points let each crate own its phase. The downside (the frame
driver must call four methods in order) is a feature: it makes the
phase boundaries visible and grep-able.

### Phase-guards as macros at every write call site

Rejected. Macros at call sites couple the assertion code to
syntactic positions in every write function; future write paths
(future G5 spawn, future signal-update fast paths) would have to
remember to invoke the macro. A `pub(crate) fn assert_paint_safe`
called as the first line of each write is more visible in code
review and adds no proc-macro complexity.

### Forbid all writes in Paint, no deferred-write queue

Rejected. The Paint-phase `dispatch_action` use case is legitimate
(telemetry / analytics during paint). GPUI defers; sigil mirrors.
A "release build silently drops the action" behaviour is the worst
of both worlds (no panic to fix, no work done). Either panic or
defer; we choose defer for actions and panic for direct mutations,
matching GPUI's split.

### Defer all writes (signal.set, entity.update, etc.), not just actions

Rejected. Direct state mutations from Paint code are almost always
bugs — the widget is computing in paint when it should be
computing in Effect. Deferring a buggy mutation produces
correct-looking next-frame state from buggy code, which masks the
bug indefinitely. Actions are the indirection layer (the entire
point of DR-046); they exist precisely to express "intent now,
mutation later," so deferring them is semantics-preserving.
Asymmetric deferral is the right shape.

### Reactive `Signal<DrawPhase>` so widgets can subscribe to phase changes

Rejected. Phase changes are not data the application reacts to as a
stream. A widget that paints differently per phase reads the phase
inside its paint method (`cx.is_painting()`); a widget that runs
an effect during Effect just runs an effect (effects already run
in Effect, by construction). Signalising the phase would force
observers to fire on every frame's phase transitions, which is
60+ Hz of fan-out for no information gain.

### Phase enum with five variants (`Effect | Layout | Paint | Composite | Idle`)

Rejected for G4, deferred. DR-034 distinguishes Paint (per-layer
render-to-texture) from Composite (final compose to backbuffer).
The two phases are sequential pieces of one GPU render pass; from
the reactive runtime's perspective they are both "writes are
forbidden, reads are allowed." Collapsing them into Paint matches
GPUI and keeps the enum smaller. If a future consumer needs to
distinguish them at the reactive seam, the enum extends — but
today no consumer does.

### Phase as bit flags (`PhaseFlags::PAINT | PhaseFlags::COMPOSITE`)

Rejected. Phase is a state machine, not a set. Composite is not
"also Paint"; it is "Paint is over, this is the next step." Flags
encode set-membership; an enum encodes state.

### `assert_paint_safe` panics in release too (no `debug_assert`)

Rejected. Release-build panics on a write-during-Paint mean a
single buggy widget can crash a shipping app in front of a user.
The deferral path (decision 5) is the safety net; debug-assert is
the developer signal. The split is industry-standard for this kind
of invariant (matches GPUI; matches React's "setState during
render" warning).

### Per-write-path `phase: DrawPhase` parameter

Rejected. Threading phase through every reactive op's signature
infects every consumer with a parameter they do not own. The
thread-local field is the right place — the runtime knows the
phase, the caller does not.

---

## Open questions

These are explicitly **not** resolved in this DR. They land in
follow-up DRs or W3.2-impl implementation discovery.

1. **W3.2b — frame driver wiring.** Who calls `begin_effect` /
   `begin_layout` / `begin_paint` / `end_frame` in the sigil
   process? The natural home is the future top-level `App` struct
   in `crates/sigil/`. The exact shape (sync driver, async driver,
   60 Hz fixed-rate, vsync-driven) is W3.2b.

2. **DR-048 / G5 — `Task<T>` + completion-handler phase
   contract.** Cites this DR for "completion handlers run in
   Effect, never Paint." Scheduled after G3/G4 close.

3. **`deferred_writes` queue size bounds.** Today the queue is
   unbounded `Vec`. A pathological widget that dispatches an
   action every paint instruction (10 000 per frame) would
   accumulate 10 000 entries per frame, drained at `end_frame`.
   Not a real consumer pattern, but defensible to cap the queue at
   a fixed size (e.g. 1024) and panic-in-debug / drop-with-trace
   in release on overflow. Decision: defer to W3.2-impl
   implementation discovery; if a real consumer trips a sane
   bound, lock it in a follow-up DR.

4. **`Context::set_global` during Paint — defer or panic?**
   Decision 5 currently panics-in-debug / no-ops-in-release for
   `set_global`, matching the other direct-mutation paths. There
   is an argument that globals are coarse enough that a
   Paint-phase `set_global` (e.g. recording last-painted theme
   into a global cache) is the same intent as a Paint-phase
   action. Provisional: panic + no-op. Re-evaluate if a real
   consumer needs deferral.

5. **Phase enforcement in observer callbacks running inside an
   action handler chain (decision 8 cycle).** Today the drain
   loop iterates until both queues are empty; a runaway cycle
   trips `MAX_DRAIN_PASSES = 16`. The exact cap value is a
   judgement call — 16 is conservative. W3.2-impl picks the
   final number based on the test fixtures.

6. **Phase visibility in the trace hook.** The existing
   `__trace_test_hook` interface emits effect-run events. Should
   phase transitions also emit trace events for offline analysis?
   Provisional: yes, behind the existing `SIGIL_TRACE` env var.
   Deferred to W3.2-impl.

7. **`Runtime::end_frame` is a write boundary itself.** Replaying
   a deferred action in `end_frame` calls back into
   `dispatch_action`, which re-checks `phase`. At `end_frame` the
   phase is Paint (we have not yet transitioned to Idle). We
   resolve this by transitioning to Effect first, draining the
   queue, then transitioning to Idle:
   `Paint → Effect (drain) → Idle`. The four-method API still
   shows four calls; the third call (`end_frame`) internally
   collapses the Paint → Effect → Idle transition. Documented in
   the API sketch.

8. **GPUI's `WindowContext` parity.** GPUI has a richer
   `WindowContext` that knows about the current window, focused
   handle, paint state, layout request, etc. Sigil's
   `Context<'_, T>` is entity-scoped, not window-scoped. The
   phase field on the runtime is window-agnostic — multi-window
   sigil (Phase X) will need to extend phase to be per-window.
   Out of scope for G4; documented as a forward concern.

---

## References

- **DR-040** — sigil thesis lock (mirror GPUI's validated
  substrate).
- **DR-045** — G2 entity model (the entity-update write seam G4
  asserts on).
- **DR-046** — G3 actions + focus (the dispatch-action write seam
  G4 asserts on, plus the deferral target).
- **DR-034** — layer pool + two-phase frame walker (the Paint
  pass G4 brackets).
- **DR-005** — reactive owner tree (the underlying signal/effect
  substrate with the existing `with_rt` borrow discipline).
- **Master dispatch plan** —
  `docs/plans/2026-05-18-parallel-dispatch-master.md` (W3.2 is
  this DR; W3.2-impl is the implementation).
- **GPUI parity matrix** —
  `docs/plans/2026-05-16-gpui-parity-integration.md`,
  framework-substrate row "Frame phases" / "Paint safety".
- **API sketch** — `docs/specs/g4-drawphase-api-sketch.md`
  (companion to this DR).
- **`crates/reactive/src/runtime.rs`** lines 192–271 —
  `RuntimeState` + `RuntimeState::new` + thread-local definition;
  G4 extends both.
- **`crates/reactive/src/signal_write.rs`** lines 89–202 —
  representative write path (`Signal::set` / `Signal::update`)
  where the assertion seam lives.
- **`crates/reactive/src/entity.rs`** lines 234–288 —
  `run_update` write entry point.
- **`crates/reactive/src/cx.rs`** lines 66–80, 126–169,
  295–361 — `Context::notify` / `Context::emit` /
  `Context::on_action` / `Context::dispatch_action` write seams.
- **GPUI reference** — `zed-industries/zed/crates/gpui` window /
  phase model (conceptually: `Phase::{Layout, Paint, ...}` on the
  window struct, write guards on every state-mutating method).
