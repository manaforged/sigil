---
id: DR-045
status: accepted
date: 2026-05-18
covers: G2 entity model + observer primitives (W2.1a design, W2.1b implementation)
depends_on: [DR-040, DR-005, DR-044]
relates: docs/plans/2026-05-18-parallel-dispatch-master.md, docs/specs/g2-entity-api-sketch.md
---

# DR-045 — G2 Entity State Model + Observer Primitives

**Status:** Accepted
**Date:** 2026-05-18
**Covers:** G2 keystone — `Entity<T>`, `Subscription`, `Context<'_, T>`,
`cx.observe`, `cx.subscribe`, `cx.notify`, `cx.observe_global`
**Depends on:** DR-040 (sigil thesis lock — "own the foundation; mirror
GPUI's validated shape"), DR-005 (reactive owner tree — the substrate
this DR builds on), DR-044 (G1 text pipeline — runs parallel to G2; no
ordering coupling)
**Unlocks:** G3 actions (focus context routes through `Entity`), G4
DrawPhase (frame invariants assert on entity-update boundaries), G5
async `Task<T>` (cancellation hangs off owner disposal which entities
participate in).

---

## Context

DR-040 commits sigil to mirroring GPUI's `Entity<T>` / `Subscription` /
`Context` / `notify()` surface directly. The substrate-parity row of the
GPUI parity plan (2026-05-16) names those four primitives as the G2
deliverable. G3/G4/G5 in Wave 3 of the parallel-dispatch master plan all
gate on G2 landing — actions need a typed handle to dispatch against,
DrawPhase needs an entity-update boundary to assert on, async tasks need
an owner-bound cancellation handle.

What already exists:

- **`sigil-reactive` owns the owner tree** (DR-005). Owners have stable
  generational `OwnerId`s, named owners with `Location::caller()`
  capture, depth-first dispose, `on_cleanup` LIFO order, batch / peek /
  cycle detection, dev-time leak detection, `OwnerHandle` for
  cross-thread identity. Main-thread-only, `!Send`, thread-local arena.
- **`sigil-kernel::Cx`** holds the node arena + a root reactive owner
  and creates per-component child owners on every `Cx::add`. This is
  the composition context, not the entity model — it sees nodes, not
  user state.
- **Signals exist** (`Signal<T>: Copy + 'static`) but signals are
  granular *fields*, not *entities*. GPUI's `Entity<T>` is a typed
  handle to a whole user-defined struct that can carry many fields,
  fire one batched `notify()` per turn, and have heterogeneous observers
  watching it. Signals do not subsume this — they are the wrong
  granularity.

What does not exist:

- **No `Entity<T>`.** There is no typed handle for "a piece of user
  state owned by the framework, observed by other entities, disposed
  when its owner scope ends."
- **No observer fan-out by entity identity.** Effects subscribe to
  signals, but there is no "observe entity `E` and react when any of
  its internal updates fire `notify()`".
- **No global registry.** GPUI's `cx.observe_global::<T>()` looks up a
  singleton `T` keyed by `TypeId`. We have nothing equivalent.

GPUI's actual surface (Zed-monorepo `gpui` crate, mirrored by `gpui-ce`
0.3.2):

```rust
let model: Entity<Counter> = cx.new(|_| Counter::default());
let _sub: Subscription = cx.observe(&model, |this, m, cx| { ... });
model.update(cx, |c, cx| { c.value += 1; cx.notify(); });
```

G2's job is to land the sigil equivalent without forking the substrate.
The `sigil-reactive` owner tree is GPUI-shaped already (per-component
owners, cascading disposal, named handles); entities slot in as
arena-backed payloads whose drop is wired into owner disposal.

---

## Decision

Land seven decisions, each frozen for downstream G3/G4/G5 dispatch.

### 1. Home crate — `crates/reactive/`

`Entity<T>` lives in `sigil-reactive`, not `sigil-kernel`. Module path:
`crates/reactive/src/entity.rs`, re-exported from the crate root.

Rationale: entity lifecycle *is* reactive lifecycle. Disposal cascades
through the owner tree (DR-005); observation fan-out reuses the
effect-scheduling machinery already in `runtime.rs`. `sigil-kernel`
sees nodes — visual things in the tree — not user state. Putting
entities in kernel would force kernel to depend on its own observers,
turning kernel into the App-singleton anti-pattern. Reactive is the
right place; kernel imports it (already does, for `Owner`).

### 2. `Entity<T>` shape — `Copy` arena handle, generational, boxed payload

```rust
pub struct Entity<T: 'static> {
    key: EntityKey,  // generational, like SignalKey
    _not_send: PhantomData<*const T>,
}
```

Mirrors GPUI's `Entity<T>` exactly: a typed `Copy` handle, not the
value itself. The payload (`Box<dyn Any>`) lives in a thread-local
`entities: Arena<EntitySlot>` field added to the existing
`RuntimeState`. `EntitySlot` carries `{ owner: OwnerKey, value:
Box<dyn Any>, subscribers: SmallVec<[ObserverKey; 4]>, name:
&'static str, location: &'static Location }`.

Why mirror GPUI exactly:

- `Copy` handle + arena payload is the same shape `Signal<T>` already
  uses in this crate. The arena infrastructure is reusable.
- Generational keys give stale-handle panics with actionable messages,
  matching DR-005 addendum item 7.
- Boxed payload behind `dyn Any` keeps `Entity<T>` itself
  monomorphization-light — the arena is one type, downcast happens at
  the access seam.
- `!Send + !Sync` per DR-005's main-thread invariant. Cross-thread use
  goes through a sendable identity (deferred — see Open questions).

### 3. Ownership lifecycle — entity is a child of `current_owner` at creation

`Entity<T>` is created inside an active owner scope via
`cx.new::<T>(|cx| T::default())` or `Entity::new(name, |cx| ...)`. The
entity is recorded in `OwnerSlot::entities: Vec<EntityKey>` (a new
field paralleling `OwnerSlot::signals`). When `dispose_owner` runs, it
walks `entities` and removes their arena slots after firing per-entity
release hooks (the entity's own observers are evicted from
`subscribers` lists on every observed peer).

No internal weak-ref. No cleanup-list entry. The owner tree already
disposes its children depth-first and runs cleanups LIFO; entities ride
the existing cascade. Specifically reuses
`crate::owner_dispose::dispose_owner` — G2 adds one branch for the
entities vector.

### 4. `Subscription` — RAII handle, drops auto-unsubscribe

```rust
#[must_use = "the subscription is dropped immediately if not bound"]
pub struct Subscription {
    key: ObserverKey,
    // !Send + !Sync
    _not_send: PhantomData<*const ()>,
}

impl Drop for Subscription { fn drop(&mut self) { /* remove key from
   per-entity subscribers list */ } }
```

Mirrors GPUI's `Subscription` exactly: dropping the handle severs the
observation. **Additionally**, every `Subscription` registers an
`on_cleanup` against the current owner that fires the same removal —
so if the bearer forgets to store the handle (a `_sub` pattern or
`let _ = cx.observe(...)`), the subscription is also cleaned up when
its enclosing owner disposes. Two paths, idempotent: explicit drop
*and* owner cascade both remove the same `ObserverKey`. Re-entry is a
no-op.

This is a deliberate divergence from GPUI in one direction (we add the
owner-cascade safety net) but not the other (we still honor explicit
`drop(sub)` immediately). It eliminates the `Subscription` leak class
that GPUI users hit when they bind to `let _ =` and lose the handle.

### 5. `notify()` propagation — batched, drained at the end of the dispatch turn

`cx.notify()` inside an `entity.update(cx, |t, cx| { ... })` closure
schedules observers in `RuntimeState::pending` (the same queue that
signal writes already use). The queue drains at the end of the
outermost `update` or `Runtime::scope` frame, in arrival order, with
duplicate-suppression via `pending_set` (already exists for effects).

Why batched, not immediate:

- Matches GPUI exactly. GPUI batches because synchronous fan-out from
  inside `update` causes the same "modify entity mid-iteration"
  hazards we already see with signal writes.
- Reuses the existing batched effect scheduler in
  `crate::signal_write` + `crate::effect_run`. Two write paths
  (signal-set and entity-notify) sharing one queue keeps the
  scheduling story simple and the priority-inversion question moot.
- Synchronous fan-out would require re-entrant `RefCell` borrows
  through user code, which is exactly what `with_rt` is structured
  to avoid (per the existing crate doc).

Observers that read the entity inside their handler see the post-write
state (the update closure has completed before the queue drains).

### 6. `observe_global<T>()` — `TypeId`-keyed map on `RuntimeState`

```rust
// Stored on RuntimeState (next to entities arena):
globals: AHashMap<TypeId, EntityKey>,
```

`cx.set_global::<T>(value)` boxes the value as an entity (parented to
the root owner of the current `Runtime::scope`) and stores its
`EntityKey` under `TypeId::of::<T>()`. `cx.observe_global::<T>(|this,
cx| { ... })` looks up the key and registers an observer against it.
Replacing the global (`cx.set_global` called twice on the same `T`)
disposes the old entity, fires its observers' on_drop, and installs the
new one. Globals are *entities under the hood* — same observer
plumbing, same notify path, free.

Storage detail (sketched, implementation deferred): one `AHashMap` is
fine at G2-tier scale (≤ 100 globals expected in a normal app).
Migrating to a flat `TypeMap` with linear probing is a perf-only
follow-up if the global count ever crosses the AHashMap break-even.
Not a blocker for W2.1b.

### 7. `Context<'a, T>` — borrow shape

```rust
pub struct Context<'a, T: 'static> {
    rt: &'a mut RuntimeState,
    entity: EntityKey,
    _t: PhantomData<&'a mut T>,
}
```

A `Context<'_, T>` is the borrow handed to every update closure and
observer callback. It carries `&mut RuntimeState` (the runtime borrow
we already take in `with_rt`) plus the `EntityKey` whose payload is
"the entity this context is currently inside."

Public methods on `Context<'_, T>`:

- `cx.notify(&self)` — schedule observers of `self.entity` for drain.
- `cx.new<U>(|cx| U::default()) -> Entity<U>` — spawn a child entity
  parented to the current owner (which during an `update` is the
  owning entity's owner, not the original creator).
- `cx.observe<U>(&Entity<U>, F) -> Subscription` — register observer.
- `cx.subscribe<U, Ev>(&Entity<U>, F) -> Subscription` — typed event
  channel (sub-channel of observe with a payload).
- `cx.observe_global<G>(F) -> Subscription`.
- `cx.set_global<G>(value: G)` / `cx.global<G>() -> &G` /
  `cx.global_mut<G>(F)`.

Borrow story: `&mut RuntimeState` is exclusive for the duration of the
closure, which compiles given sigil-reactive's existing `with_rt`
borrow rules (one re-entrant borrow only via `with_rt` itself, and
user code is structurally outside that re-entrancy). The `_t:
PhantomData<&'a mut T>` is the marker that says "this context is
inside an update on a `T`-typed entity" — used by `cx.notify` to know
which entity's subscribers to wake.

Inside an observer callback the signature is `fn(&mut This, &Entity<U>,
&mut Context<This>)` — the observer's own entity is the `Context`
parameter, the observed entity is the second arg, and `This` is the
observer's payload reached via `cx.entity_mut()` (which is the
short-circuit downcast of `Context<'_, T>` to `&mut T`). Mirrors GPUI's
`(this: &mut Self, observed: &Entity<U>, cx: &mut Context<Self>)`
exactly.

---

## Consequences

- **`crates/reactive/` gains one module (~250 LoC budget)** plus
  per-owner-slot entity tracking and per-runtime entity arena +
  global-typemap fields. No widget call site moves.
- **`Entity<T>` becomes the public seam every long-lived state lives
  behind.** Plain `Signal<T>` is for granular fields; `Entity<T>` is
  for whole structs. Authors choose based on whether they need
  observation by other entities (entity) or just by effects (signal).
- **G3 actions consume `Entity<T>` as the dispatch target.** Action
  registration keys against the focused entity's `EntityKey`; the
  focus-context resolver walks the entity's owner ancestry. G3 design
  DR (forthcoming) inherits this contract.
- **G4 DrawPhase can assert on entity-update boundaries.** Paint-phase
  mutations to an entity trip a debug assert because the DrawPhase
  state machine sees `update()` as a write and Paint disallows writes.
  G4 DR will cite this DR for the assertion seam.
- **G5 `Task<T>` cancellation hangs off the entity owner.** `Task`
  spawned inside `entity.update` is parented to the entity's owner —
  entity dispose ⇒ task cancel. Free, falls out of the owner-cascade
  design.
- **`Subscription` leak class is structurally eliminated.** The
  owner-cleanup safety net (decision 4) means dropped subscriptions
  cannot outlive their creating scope even if the bearer mishandles
  the handle. GPUI does not have this — it relies on `#[must_use]`
  alone.
- **Bench surface lands in W2.1b.** Criterion benches in
  `crates/reactive/benches/entity.rs`: `entity_update_no_observers`
  (target ≤ 30 ns/op), `entity_notify_drain_4_observers` (target ≤
  400 ns/op), `observe_global_hit` (target ≤ 60 ns/op). Numbers
  derived from existing signal-op bench results (~25 ns set, ~50 ns
  get) scaled by the extra TypeId / arena lookups; will be re-fit on
  first real bench.

---

## Alternatives considered

### Home crate: `crates/kernel/` instead of `crates/reactive/`

Rejected. Kernel sees nodes; entities are user state that does not
need to be in the node tree. Putting `Entity<T>` in kernel would force
either (a) kernel to import its own reactive observer machinery,
duplicating logic already in reactive, or (b) reactive to depend on
kernel for the entity type, inverting the current dep direction
(kernel → reactive). The boring answer — entities live next to the
arena machinery they reuse — is correct.

### `Entity<T>` as `Rc<RefCell<T>>` newtype

Rejected. Two problems: (a) `Rc<RefCell<T>>` is not `Copy`, so every
observer callback signature would carry `&Entity<T>` instead of
`Entity<T>` and the API diverges from GPUI; (b) interior mutability
through `RefCell` re-introduces the borrow-stack hazards the existing
`with_rt` discipline structurally avoids. Generational arena handles
are the existing pattern in this crate; entities reuse it.

### Immediate `notify()` fan-out

Rejected. Three reasons: (a) GPUI batches, and we are mirroring GPUI;
(b) immediate fan-out requires re-entrant borrows through user code,
which the `with_rt` discipline forbids; (c) the existing
`signal_write` path already batches via `pending` + `pending_set`,
and entity-notify slots into that queue for free. Adding a second,
synchronous dispatch path would split the scheduling story.

### `cx.observe` returns `()` and stores the subscription on the
current owner implicitly

Rejected. The Subscription handle is the explicit unsubscribe seam.
GPUI returns it; removing it would force unsubscribe-before-owner-drop
to go through a separate API. We add the owner-cascade as a safety
net but keep the explicit handle. Both paths converge on the same
`ObserverKey` removal, idempotent.

### `observe_global<T>` as a separate `Resource<T>` type, not an entity

Rejected. The `cx.observe_global<T>` semantics are identical to
`cx.observe(&entity)` once `T` is wrapped as an entity. Reusing the
entity path means one observer-fan-out implementation, one set of
debug tools, one dispose cascade. Introducing a parallel `Resource<T>`
shape would duplicate the machinery for no behavioral win.

### `Context<'a, T>` as `Context<'a>` (untyped, with separate `T`
parameter on methods)

Rejected. The phantom `T` on Context is what lets `cx.notify()` know
which entity to wake without an explicit argument. GPUI's signature is
`(this: &mut T, cx: &mut Context<T>)`; the `T` on Context is
load-bearing. Untyped Context would require `cx.notify::<T>()` at
every call site, which diverges ergonomically from GPUI.

---

## Open questions

These land in DR-046+ or are deferred to W2.1b implementation discovery
without DR-blocking.

1. **`SendableEntity<T>` — cross-thread identity.** GPUI has a sendable
   weak handle (`WeakEntity<T>`) usable on background threads. The
   equivalent is the same pattern as `SendableSignal` (DR-005 + the
   current `into_sendable` impl), but the design has one extra question:
   do we want a `WeakEntity<T>` that fails-soft on dispatch (matches
   `OwnerHandle` semantics in DR-005 §3) or fails-hard? Answer:
   fails-soft, but the W2.1b implementer can defer the type entirely if
   no G3/G4/G5 caller needs it on day one.

2. **`set_global<T>` re-set policy.** When `cx.set_global::<T>(new)` is
   called and `T` already has a global, do we (a) dispose the old
   entity immediately, (b) keep both and let the old observers race, or
   (c) panic? Decision: (a) — dispose old, install new, fire on_drop
   for old. Resolved here for completeness, no DR-046 needed.

3. **Observer ordering across multiple entities firing in one batch.**
   When entities A and B both `notify()` inside the same `update`
   closure, do observers see A's observers fire first, B's first, or
   interleaved by registration order? Decision: arrival order at the
   `pending` queue, same as signals. Existing crate behavior;
   inherited.

4. **Re-entrant `update` on the same entity.** Calling `b.update(cx,
   ...)` from inside `a.update(cx, ...)` is allowed. Calling
   `a.update(cx, ...)` from inside `a.update(cx, ...)` panics with
   "entity already locked for update," same shape as the existing
   `UpdateInProgress` guard in `crate::signal_write`. Resolved here.

5. **`Entity<T>` `Debug` impl.** Carries name + type name + arena
   index/generation, matching `Signal<T>::Debug`. Implementation
   detail; resolved.

6. **DR-046 (forthcoming): G3 action dispatch + focus context.**
   Consumes this DR's `Entity<T>` as the dispatch target. Schedule:
   after W2.1b lands.

7. **DR-047 (forthcoming): G4 DrawPhase state machine + entity-update
   assertion seam.** Cites this DR for the write-boundary location.

8. **DR-048 (forthcoming): G5 `Task<T>` + entity-owner-cancellation
   contract.** Cites this DR for the cancellation seam.

---

## References

- **DR-040** — sigil thesis lock (own the foundation, mirror GPUI).
- **DR-005** — reactivity owner tree (the substrate this DR builds on).
- **DR-044** — G1 owned text pipeline (parallel track, no coupling).
- **Master dispatch plan** — `docs/plans/2026-05-18-parallel-dispatch-master.md`
  (W2.1 is this DR; W2.1b is the implementation).
- **GPUI parity matrix** — `docs/plans/2026-05-16-gpui-parity-integration.md`,
  framework-substrate row "Entity ownership" + "Reactive observation".
- **API sketch** — `docs/specs/g2-entity-api-sketch.md` (companion to
  this DR).
- **`crates/reactive/src/owner.rs`** — owner-tree API surface this
  builds on.
- **`crates/reactive/src/runtime.rs`** — `RuntimeState` + arenas + the
  `pending` queue this DR extends.
- **`crates/reactive/src/signal.rs`** — `Signal<T>` shape this DR
  parallels for `Entity<T>`.
- **GPUI reference** — `zed-industries/zed/crates/gpui` (pre-1.0,
  internal) + `gpui-ce/gpui-ce 0.3.2` (community extraction).

---

## Amendment 2026-05-18 — Entity arena eviction on owner disposal

W3.3-impl (G5 `Task<T>`, commit `d4dbf25`) surfaced a hole in the
disposal cascade: `crate::owner_dispose::dispose_owner` removes the
owner slot from `rt.owners` but **does not evict the owner's entities
from `rt.entities`**. The owner's `entities: Vec<EntityKey>` (decision 3
above, runtime.rs:138) was populated on `Entity::new` but never walked
during dispose.

Consequences observed in W3.3-impl:

- **Memory leak.** Every disposed entity's `EntitySlot` (payload box,
  subscribers SmallVec, name, location) stays in the arena for the
  process lifetime. No code path removes it.
- **Async safety hole.** `AsyncContext::update` and `Entity::update`
  both gate on `rt.entities.contains(key)` (see `async_cx.rs` + the
  `disposed_entity` diagnostic in `entity.rs:178/243`). Because the
  entry is never removed, the gate never trips: a Task holding an
  `AsyncContext<U>` after its owner disposed can still mutate the
  entity's payload through the back door. Logically the entity is gone;
  mechanically the data is still there.
- **Test gap.** W3.3-impl could not write
  `update_after_entity_gone_returns_err` because the path is
  unreachable from any user code. The test was reshaped around
  dispatcher-closed semantics instead.

This amendment closes the hole.

### A1 — `dispose_owner` evicts the owner's entities

`dispose_owner` (`crates/reactive/src/owner_dispose.rs`) gains a new
phase **between current phases 3 (signals/effects) and 4 (detach +
remove owner slot)**:

```text
Phase 3.5: Evict the owner's entities.
  - Take `owner.entities: Vec<EntityKey>` (mem::take, same pattern as
    phase 2's `owner.cleanups` and phase 3's `owner.signals/effects`).
  - For each EntityKey:
      a. Look up the entity's `subscribers` and `event_subs` SmallVecs.
      b. For every ObserverKey in those lists: walk the observer's
         registration site (the observer entity's own subscriber index
         on the *observed* entity — already a `SmallVec<[ObserverKey]>`
         on EntitySlot) and remove the back-edge. Remove the
         ObserverSlot from `rt.observers`. This is the inverse of the
         registration step in `entity_observer.rs::register_observer`.
      c. Walk `rt.globals: Vec<(TypeId, EntityKey)>` and `swap_remove`
         any entry whose value matches this EntityKey (a global
         entity's owner is the root scope; same-EntityKey-twice cannot
         happen because of the generational key, but the entry must
         still go).
      d. `rt.entities.remove(entity_key)` — this drops the boxed
         payload and frees the slot. The arena's generation counter
         increments per `Arena::remove` (see `arena.rs`); see A2.
```

The implementation should reuse the same `take_owner_resources` /
`remove_effects_and_signals` shape currently used for signals + effects.
A new helper `evict_owner_entities(rt: &mut RuntimeState, entities:
Vec<EntityKey>)` keeps the function-size cap.

**Plain-subtree fast path.** `try_dispose_plain_subtree` (the
no-cleanups-no-signals-no-effects fast path) must additionally check
`owner.entities.is_empty()`. If the entities list is non-empty the fast
path bails to the slow path. Add `entities.is_empty()` to **both** the
single-owner check and the descend-then-bulk-remove check.

### A2 — Generational key ABA hardening (already in place; document it)

The arena (`crates/reactive/src/arena.rs`) is already a generational
slot store (`Key<T>` carries `index` + `generation`; `remove` increments
the slot's generation, so re-insertion at the same index produces a
distinct key). No new infrastructure is needed for ABA safety —
`EntityKey = Key<EntitySlot>` inherits this. Holding a stale
`Entity<T>` whose slot has been recycled is structurally safe: the
arena's `get(key)` returns `None` when generations mismatch, and the
existing `disposed_entity` diagnostic fires.

This amendment **does not introduce slotmap or any new versioning**;
it documents that the existing arena already provides what A1's
removal needs. No DR-005-level slotmap migration is implied.

### A3 — `EntityGone` semantics, written down

For `AsyncContext<U>` (`crates/reactive/src/async_cx.rs`) and
`Entity::update`, the post-amendment contract reads:

> An entity is **disposed** when `dispose_owner` ran on the owner that
> created it. After disposal the entity's slot is removed from
> `rt.entities` and the slot's generation has incremented. Any
> subsequent access through a stale `Entity<T>` / `EntityKey` returns
> `None` from `rt.entities.get(key)`.
>
> - `Entity::update` and `Entity::read` panic with the
>   `crate::diagnostic::disposed_entity` message. This is the
>   synchronous, on-main-thread surface; a stale handle is a logic bug
>   the author should fix.
> - `AsyncContext::update` and `AsyncContext::update_entity` return
>   `Err(EntityGone)`. This is the asynchronous, off-thread surface; a
>   stale handle is the *expected* outcome when an entity's owner
>   disposed between spawn and re-entry.
> - Subscribers receive **no** explicit `Disposed` event today
>   (decision 5 above batched-notify path covers `notify()` only). A
>   future `Subscription::on_drop` hook is open question 9; this
>   amendment does not add one.

`EntityGone` (already exists at `crates/reactive/src/task.rs`) keeps
its zero-sized shape; no new variants.

### A4 — Migration shape

- **No public API change.** `dispose_owner` is `pub(crate)`; consumers
  call `Owner::dispose` (sync) or rely on owner-cascade. Both paths
  inherit the new eviction transparently.
- **Tests that relied on disposed-entity data still being readable
  break.** Any test that calls `dispose_owner` then `entity.read(...)`
  expecting the value to survive is asserting the bug, not the
  contract. Such tests must move to either (a) holding the
  `Entity<T>` only inside a child scope that re-runs, or (b) asserting
  the panic via `std::panic::catch_unwind` + the new diagnostic.
- **W3.3-impl re-writes the missing test.**
  `update_after_entity_gone_returns_err` becomes writable:

  ```text
  - Build entity E inside a child scope C.
  - Spawn a Task on E that posts `acx.update(...)` after a delay.
  - Dispose C (which evicts E from rt.entities).
  - Drain the dispatcher.
  - Assert the update closure's outer Result is Err(EntityGone).
  ```

- **`rt.globals` walk on every dispose** is O(globals × entities-being-
  evicted). Globals are bounded by `≤ 100` at G2-tier scale (decision
  6); the loop is a vec scan, not a hash lookup. Acceptable;
  re-evaluate if globals ever cross 1k.

### Downstream implementation tasks

Queued as follow-ups for the next reactive-implementer dispatch:

- `crates/reactive/src/owner_dispose.rs` — add `evict_owner_entities`
  helper + phase 3.5; update `try_dispose_plain_subtree` to bail when
  `owner.entities` non-empty.
- `crates/reactive/src/entity_observer.rs` — expose / reuse the
  back-edge removal logic so phase 3.5 can call it per observer.
- `crates/reactive/tests/owner_dispose.rs` (or extend an existing
  file) — assert (a) entity slot removed from arena after owner
  dispose, (b) `Entity::update` post-dispose panics with the
  documented message, (c) globals walk removes the disposed global.
- `crates/reactive/tests/task.rs` —
  `update_after_entity_gone_returns_err` lands here, per A4 outline.

### Rejected alternatives

- **Mark-but-don't-remove (tombstone the slot).** Same memory leak,
  one more bit per slot, no win.
- **Run eviction lazily on next `get`.** Fragile; observer fan-out
  walks `subscribers` lists that hold dead keys, branches would need
  per-iteration aliveness checks. Eager eviction is simpler and the
  cost falls on the disposing scope, not the hot read path.
- **Move eviction into `Drop` on `EntitySlot`.** The arena owns the
  slot; running user-visible cleanup on `Drop` re-entrantly borrows
  `with_rt` and re-introduces the borrow-stack hazard the crate is
  structured to avoid. Eviction stays explicit, called once from
  `dispose_owner`.

