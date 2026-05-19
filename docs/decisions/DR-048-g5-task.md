---
id: DR-048
status: proposed
date: 2026-05-18
covers: G5 `Task<T>` + `cx.spawn` + `AsyncContext` (W3.3 design, W3.3-impl implementation)
depends_on: [DR-040, DR-045, DR-046, DR-047, DR-007, DR-005]
relates: docs/plans/2026-05-18-parallel-dispatch-master.md, docs/specs/g5-task-api-sketch.md
---

# DR-048 — G5 `Task<T>` + `cx.spawn` + Async Re-Entry Context

**Status:** Proposed
**Date:** 2026-05-18
**Covers:** G5 keystone — `Task<T>` polymorphic handle, `Context::spawn`,
`AsyncContext` re-entry token, completion-routing through the existing
`Dispatcher` MPSC, RAII-cancel-on-drop with `.detach()` opt-out, owner-
cascade cancellation, deferred-completion drain in `begin_effect`.
**Depends on:** DR-040 (sigil thesis lock — mirror GPUI's validated
substrate), DR-045 (`Entity<T>` / `Context<'_, T>` — the re-entry target),
DR-046 (action dispatch — Tasks land in handler bodies), DR-047 (DrawPhase
— completion handlers run in Effect, never Paint), DR-007 (one Tokio
runtime per app — the executor G5 sits on), DR-005 (owner tree — the
cancellation spine Tasks parent against).
**Unlocks:** AI-track agent-loop primitives (AI2 — every tool call is a
`Task`), the host engine live-reload (FX4 — hot-swap state-preserving asyncs),
every widget that fetches data (combo-box autocomplete, file tree,
diagnostics).

---

## Context

DR-040 commits sigil to mirroring GPUI's validated substrate. DR-045
landed `Entity<T>` + `Context<'_, T>`. DR-046 landed typed actions +
focus. DR-047 landed the DrawPhase state machine and pre-locked one
piece of G5: **task completion handlers run in Effect phase, never
Paint** (DR-047 §7). G5 is the substrate piece that lets a widget say
`cx.spawn(async { fetch_data().await })` and have the completion run
cleanly back in the entity's reactive context, with cancellation tied
to the entity's lifetime.

What already exists (full picture, read before designing):

- **`sigil-reactive`** — `Entity<T>` + `Context<'_, T>` + observer
  fan-out + owner cascade (DR-045), action dispatch + focus stack
  (DR-046), DrawPhase enum + write-during-Paint asserts + deferred-write
  queue (DR-047, landed on `a4b022a`). The runtime is thread-local,
  `!Send`, with `with_rt` borrow discipline.
- **`sigil-async`** — DR-007's concrete adapter. Owns one
  `tokio::runtime::Runtime` per `App`. `AsyncRuntime: Scheduler`
  exposes `spawn(owner, future)` → `JoinHandle<()>`, an MPSC
  `Dispatcher` cloneable to background tasks, and `drain()` that runs
  on the main thread per `sigil-hal::Scheduler::drain`. Background
  cancellation is owner-disposal-driven via an `on_cleanup` hook
  registered at spawn time + a belt-and-braces `owner_tasks` registry
  swept at every drain. The public surface today is two functions:
  `dispatch_to_owner` (cross-thread main-thread trampoline) and
  `spawn_to_signal` (returns `Signal<Status<T, E>>`).
- **`sigil-hal::Scheduler`** — the trait `sigil-async` implements, with
  associated types `OwnerHandle`, `SpawnTask`, `Error`. Tests stub it.
- **`Status<T, E>`** — the three-state enum (`Pending | Ok(T) |
  Err(E)`) that `spawn_to_signal` writes through.

What does not exist:

- **No `Task<T>` type.** Callers today write `spawn_to_signal` and
  consume a `Signal<Status<T, E>>`. That is the value-stream shape;
  GPUI's `Task<T>` is the imperative-handle shape. Both are useful;
  G5 lands the imperative one without removing the reactive one.
- **No `Context::spawn` method.** Callers reach `sigil-async`
  directly. GPUI's surface is `cx.spawn(|cx| async move { ... })`
  with the closure-over-`cx` pattern; sigil entities currently have
  no equivalent.
- **No `AsyncContext` / re-entry token.** GPUI's spawned closure
  receives an `AsyncWindowContext` (or `AsyncAppContext`) that
  carries a `WeakEntity<Self>` and a way to call
  `cx.update(weak, |this, cx| ...)` back on the main thread. Sigil
  has `dispatch_to_owner`, which is the building block, but not the
  ergonomic `cx.update` shape.
- **No completion-handler phase contract enforced.** DR-047 §7 pre-
  locked that completions must run in Effect, never Paint. The seam
  for that is "drain completion callbacks at the top of
  `Runtime::begin_effect`" — but the queue does not exist yet.

What this DR is **not**:

- Not a re-architecture of `sigil-async`. The crate stays. `Task<T>`
  is built **on top** of `AsyncRuntime` + `Dispatcher` +
  `with_owner_by_id`, not a replacement. Two surfaces co-exist:
  reactive (`Signal<Status<T, E>>`) and imperative (`Task<T>`).
- Not an executor change. DR-007 picked Tokio; G5 does not revisit.
- Not a Send-ness re-litigation of the reactive runtime. The runtime
  is `!Send`; that is locked. G5's choice is what the *future body*
  is, given the runtime is `!Send`.

### Reading the existing repo

- `crates/async/src/runtime.rs` — `AsyncRuntime: Scheduler`,
  `Dispatcher`, `owner_tasks` registry, `Scheduler::spawn` with
  `on_cleanup`-driven cancellation.
- `crates/async/src/dispatch.rs` — `dispatch_to_owner` and
  `spawn_to_signal`. The latter is the closest existing analog to
  what `Task<T>` does: spawn a future on tokio, write back to main
  on completion via the MPSC. G5's `Context::spawn` reuses the same
  shape; the only delta is the user-facing handle (`Task<T>` instead
  of `Signal<Status<T, E>>`).
- `crates/reactive/src/cx.rs` lines 41-49 — `Context<'_, T>` is a
  token, not a `&mut RuntimeState` borrow. `Context::spawn` follows
  the same pattern.
- `crates/reactive/src/runtime.rs` — `RuntimeState` grew `phase` and
  `deferred_writes` for DR-047. G5 adds one more field
  (`pending_completions`) following the same pattern.
- DR-047 §5 — deferred-write queue carries `DeferredKind::Action(...)`
  today and §7 reserves `DeferredKind::TaskComplete(...)` for G5. We
  use a sibling queue (not the same vec) — see Decision 4.

---

## Decision

Nine decisions, each frozen for the W3.3-impl dispatch.

### 1. Executor — Tokio, via existing `sigil-async::AsyncRuntime`

The executor is Tokio. This is not re-decided in G5; DR-007 (accepted
2026-04-14) picked one multi-thread Tokio runtime per `App`, with
`tokio::spawn` for background work and an MPSC dispatcher to the main
thread. `sigil-async` ships the concrete adapter today. G5's
`Context::spawn` is plumbing on top: route the user's future through
`AsyncRuntime::tokio_handle().spawn(...)`, route the completion back
through the existing `Dispatcher` MPSC.

This composes with everything DR-007 already gave us:

- One runtime per app (not per window, not per entity).
- Owner-disposal-driven cancellation already exists at the
  `Scheduler::spawn` level via an `on_cleanup` hook + `owner_tasks`
  registry.
- Background tasks already carry an `OwnerHandle` and re-enter the
  main thread via `Dispatcher::send`.
- `EventLoopProxy::wake_up()` already wires the main thread to wake
  on enqueue.
- Headless tests already stub the scheduler via `sigil-hal::Scheduler`.

Why not revisit (smol / async-executor / single-threaded queue):

- **Smol / async-executor**: DR-007 considered and rejected both for
  proven-at-scale and winit-integration reasons; nothing in G5's
  problem statement changes the calculus. Re-deciding executors in
  G5 would invalidate the live `sigil-async` adapter, the
  `Dispatcher` MPSC, every `OwnerHandle` semantic, and the
  `spawn_to_signal` test surface — all to gain nothing G5 needs.
- **Single-threaded LocalSet / BYO queue**: would force the future
  body to be `!Send`, which lets the future hold `Entity<T>` handles
  directly. The cost is that **every** async operation a user
  awaits (`reqwest::get`, `tokio::fs::read`, anything from the
  ecosystem) is `Send`-bounded. A `!Send` runtime poisons the entire
  await tree at the type level. Forcing the user to choose between
  "borrow Entity directly" and "call any existing async library" is
  worse than the GPUI shape ("borrow nothing; re-enter via
  `cx.update`"). GPUI made this trade explicitly and shipped; we
  mirror.
- **Pure `futures::executor`**: not multi-threaded — we are already
  multi-threaded via DR-007. Downgrade with no win.

The executor choice is **locked**, and locked the same way DR-007
locked it. G5 inherits the kill criteria.

### 2. Send-ness — `Task<T>` futures are `Send + 'static`; `T: Send + 'static` too

```rust
impl<U: 'static> Context<'_, U> {
    pub fn spawn<F, T>(&mut self, build: impl FnOnce(AsyncContext<U>) -> F) -> Task<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    { ... }
}
```

The future is `Send + 'static`. The output `T` is `Send + 'static`.
This is the GPUI shape and it is the only shape that composes with
the Tokio executor.

**Why not `!Send`:** DR-045 made the reactive runtime `!Send` so the
arena lives in TLS and user state needn't carry `Send`. That decision
is correct for the reactive runtime. Pushing it onto the future would
mean every `await` inside the future has to be `!Send`, which
excludes Tokio's multi-thread spawn (requires `Send`) and excludes
every `Send`-bounded async library in the ecosystem. The cost of
this asymmetry — futures must move data, not borrow Entities —
is the same cost GPUI authors learn on day one, and the workaround
is the same: capture by clone or `WeakEntity`, and re-enter the
reactive context via `cx.update(entity, ...)` (Decision 5) to
mutate.

**Why `T: Send + 'static`:** the future runs on a Tokio worker;
its output crosses the MPSC back to the main thread. The MPSC
requires `Send`. `'static` is required by every async-spawn API and
by `JoinHandle`.

Inside the closure the user has access to an `AsyncContext<U>`
(Decision 5) which carries a `Send + Sync` `OwnerHandle` and a
`Send + Sync` `Dispatcher`. The user cannot borrow `Entity<U>` or
`Context<U>` into the future — they are `!Send`. They re-enter by
calling `acx.update(entity, |this, cx| ...).await`.

### 3. Cancellation — RAII drop-cancels by default; `.detach()` opts out; owner cascade is the safety net

`Task<T>` is a Drop-cancelling handle. Dropping the handle aborts
the underlying tokio task. Authors who want to fire-and-forget call
`.detach()`, which consumes the handle and leaves the task running
under owner control.

Three layers of cancellation cooperate:

1. **Explicit drop.** `let _t = cx.spawn(...);` — the task is
   cancelled at the end of the enclosing scope. Same shape as
   `Subscription` (DR-045 §4) and `JoinHandle::abort_on_drop` in
   tokio.
2. **`.detach()` opt-out.** `cx.spawn(...).detach();` — the handle
   is consumed without aborting. The task runs to completion (or
   until its owner disposes — layer 3 still applies). This matches
   GPUI's `Task::detach()` exactly and is the common shape for
   fire-and-forget telemetry / analytics dispatches.
3. **Owner cascade.** When the entity that spawned the task is
   disposed (per DR-045 owner cascade), every task parented to that
   entity is aborted regardless of detach state. This is the
   existing `Scheduler::spawn` behaviour (DR-007 — `on_cleanup`
   registered at spawn time aborts the tokio handle), reused
   unchanged.

The three layers are independent. Authors do not have to think
about which one fires first; any of them aborts the task, and
abort is idempotent at the tokio level.

GPUI uses this exact split (Drop-cancels, `.detach()` opts out,
owner-scope cancels everything). We mirror. The owner-cascade
layer is the structural safety net for "I forgot to detach and the
task outlives my scope" — it cannot happen because the scope's
disposal aborts the task.

**Why not "explicit `.cancel()` only":** dropping a `Task<T>`
without aborting means the task runs in the background unanchored
to user intent. The author's intent at drop time was "I no longer
want this work" — silently letting it run is a footgun. Drop-cancel
is the safe default; opt-out via `.detach()` for the documented
fire-and-forget case.

**Why not "Drop is a no-op; require `.cancel()`":** every
forgotten handle leaks a background task. The owner cascade
catches the worst case eventually, but per-drop cancellation
catches it immediately. The cost (one `AbortHandle::abort()` call
in `Drop`) is negligible.

### 4. Completion routing — `pending_completions` queue on `RuntimeState`, drained at `begin_effect`

When the user's future resolves on a Tokio worker, the value `T` and
a completion-closure pointer ride the existing `Dispatcher` MPSC
back to the main thread. Today (per `spawn_to_signal`) the closure
runs the moment `Scheduler::drain` processes it. For G5 we need it
to run **in Effect phase**, never Paint (per DR-047 §7).

The seam:

```rust
// crates/reactive/src/runtime.rs
pub struct RuntimeState {
    // ... existing fields (entities, owners, focus_stack,
    //     deferred_writes, phase, ...) ...
    pub(crate) pending_completions: Vec<PendingCompletion>,
}

pub(crate) struct PendingCompletion {
    pub(crate) callback: Box<dyn FnOnce() + 'static>,
    pub(crate) caller: &'static Location<'static>,
}
```

The MPSC-arriving completion callback does **not** invoke the user's
handler directly. It pushes onto `pending_completions` instead:

```
[Tokio worker]  future.await complete → enqueue MPSC
[main thread]   Scheduler::drain processes MPSC →
                push PendingCompletion onto rt.pending_completions
[main thread]   Runtime::begin_effect → drain pending_completions in order
```

Drain protocol (top of `Runtime::begin_effect`):

```rust
// At begin_effect, BEFORE observer fan-out:
loop {
    let next = with_rt(|rt| rt.pending_completions.pop_first());
    let Some(c) = next else { break };
    (c.callback)();  // user handler runs here, may entity.update, etc.
}
// Then proceed to drain observer queue per DR-045 / DR-047 §8.
```

Pop-front order = arrival order at the queue = completion order on
the worker, which is what authors expect. The drain happens in
`Effect` so `entity.update` / `signal.set` / `cx.notify` inside the
handler are legal (DR-047 §2 only asserts during Paint).

**Why a sibling queue (`pending_completions`) and not the existing
`deferred_writes` (DR-047 §5):** the existing queue holds
`DeferredKind::Action(Box<dyn Action>)` — actions that need to be
replayed through `Context::dispatch_action` after a Paint-phase
dispatch was intercepted. Completion callbacks are *arbitrary
closures*, not boxed actions. Cramming them into the same enum
(`DeferredKind::TaskComplete(Box<dyn FnOnce()>)`) is structurally
fine — DR-047 §7 reserved that variant — but the *drain points*
differ: deferred actions drain at `end_frame`, completions drain at
`begin_effect`. Two queues, two drain seams, each named for its
purpose. Combining them would require a per-entry phase tag and a
two-pass drain; splitting them keeps each one's contract crisp.

**Why drain at `begin_effect` and not `end_frame`:** GPUI flushes
async results before observers run, so observers can read the new
state in the same frame the completion landed. Draining at
`end_frame` would defer all reactive consequences of the completion
to the next frame, doubling latency. `begin_effect` is the right
drain point.

**Re-entrant completion enqueue.** A completion handler may itself
spawn a task whose future is already resolved (instant-complete
future); the new completion lands in `pending_completions` and
drains in the same `begin_effect` loop. The loop terminates because
each iteration pops one entry; new entries enqueued by handlers
are processed in arrival order. A fixed iteration cap
(`MAX_COMPLETION_PASSES = 256`) catches the pathological case of
each completion enqueuing two more, mirroring DR-047 §8's
`MAX_DRAIN_PASSES`.

### 5. `AsyncContext<U>` — re-entry token; `update` is the main-thread trampoline

Inside the spawned future, the user holds an `AsyncContext<U>`, not a
`Context<'_, U>`. The async context is `Send + Sync + 'static`;
the reactive context is `!Send`. Two types, one name pattern.

Shape:

```rust
pub struct AsyncContext<U: 'static> {
    entity: Entity<U>,        // Copy, type-erased to EntityKey internally
    owner: OwnerHandle,        // Send + Sync — DR-007's identity-only handle
    dispatcher: Dispatcher,    // Send + Sync — DR-007's MPSC sender
    _t: PhantomData<U>,
}
```

The only way to *mutate* the spawned entity (or any entity) from
inside the future is via:

```rust
impl<U: 'static> AsyncContext<U> {
    pub async fn update<R, F>(&self, f: F) -> Result<R, EntityGone>
    where
        F: FnOnce(&mut U, &mut Context<'_, U>) -> R + Send + 'static,
        R: Send + 'static;

    pub async fn update_entity<V: 'static, R, F>(
        &self,
        entity: &Entity<V>,
        f: F,
    ) -> Result<R, EntityGone>
    where
        F: FnOnce(&mut V, &mut Context<'_, V>) -> R + Send + 'static,
        R: Send + 'static;
}
```

`update` mutates the spawning entity (the common case — "I spawned
the fetch from inside *this* entity; on completion I mutate
*this* entity"). `update_entity` mutates any other entity the
caller has a handle to (the cross-entity case — "I'm an entity that
spawned a fetch on behalf of a peer").

Mechanism: both methods push a closure through the existing
`Dispatcher::send`, with the closure's body running inside
`with_owner_by_id(self.owner, ...)` on the main thread on the next
drain (existing `Scheduler::drain` mechanism). The `.await` resolves
when the main thread completes the closure and signals via a
one-shot back-channel.

Return type is `Result<R, EntityGone>` — fails-soft. The error fires
when the entity (or its owner) was disposed between the spawn and
the update attempt. This matches `OwnerHandle::is_alive` semantics
in DR-007 — a stale handle is a documented no-op, not a panic.

```rust
pub struct EntityGone;
// Single zero-sized variant; carries Location if useful at debug time.
```

**Naming — why `AsyncContext`, not `BackgroundContext` /
`AsyncAppContext`:** sigil's existing naming convention is
`Context<'_, T>`, `Subscription`, `Entity<T>`, `Action`. The async
flavour is a parallel context type for code-running-off-thread; the
short, parallel name `AsyncContext<U>` is the smallest naming
delta from `Context<'_, U>`. GPUI uses `AsyncWindowContext` /
`AsyncAppContext`, but sigil does not have a window-vs-app
distinction at this layer (per DR-045's entity-centric model).
Singular `AsyncContext` parameterised by the entity type is the
right shape.

**Why not expose `&mut Context<'_, U>` directly across the future
boundary:** `Context<'_, U>` is `!Send`. Crossing it through a
tokio worker is a type error. The closure-based `update` API is the
trampoline that re-establishes the `!Send` context on the main
thread, which is precisely where it's safe.

**Why `AsyncContext: Clone`** (it is, by deriving from a `Send +
Sync` `OwnerHandle` + `Send + Sync` `Dispatcher` + `Copy`
`Entity<U>`): authors fan out work — `let acx2 = acx.clone();
tokio::join!(..., async move { acx2.update(...).await })`. GPUI's
`AsyncWindowContext` is `Clone` for the same reason. Cloning is
cheap (one `Arc` increment for the dispatcher waker, one
`UnboundedSender::clone`).

### 6. `Task<T>` shape — polymorphic output, `T` is whatever the future returns

```rust
#[must_use = "tasks are cancelled when dropped; call .detach() to keep running"]
pub struct Task<T> {
    state: TaskState<T>,
    _not_send_phantom: PhantomData<*const ()>,
}

enum TaskState<T> {
    Pending {
        abort: AbortHandle,
        result_rx: oneshot::Receiver<T>,
        // 'detached': abort is forgotten on drop.
        detached: bool,
    },
    // ... terminal states folded as needed by .await ...
}
```

`Task<T>` is polymorphic over the future's output. `T` may be
anything `Send + 'static` — `()`, `Result<U, E>`, or any user type.
Authors that want the GPUI-style `Status` lifecycle continue to use
`spawn_to_signal` (existing surface, still supported); authors that
want imperative `.await` use `cx.spawn(...).await`. Both are real
shapes used by GPUI authors and both ship.

`Task<T>: Future<Output = T>`. The future resolves when the
spawned work completes and the result has been shuttled back via
the oneshot. Polling drives through the oneshot.

**Errors are user-typed.** If the future returns `Result<U, E>`,
the `Task` is `Task<Result<U, E>>`. The framework does not impose
its own error union. Cancellation does **not** produce a
"cancelled" error — a cancelled task's `.await` resolves to a
dropped future / `JoinError`-equivalent which the user can choose
to surface or swallow. Practically: if you `.await` a task you
already cancelled (via `.detach()`-not-called + drop earlier), you
get `Err(TaskCancelled)`. Defined helper:

```rust
pub enum TaskAwaitError {
    Cancelled,
    Panicked,   // future body panicked (rare; surfaced for visibility)
}
```

The `.await` impl returns `Result<T, TaskAwaitError>` *only* when
the task can fail at the framework level (cancellation or panic).
The common case is `Task<T>: Future<Output = T>` where the user's
`T` already encodes their error. We achieve both by having `Task<T>`
implement `Future<Output = T>` and providing `Task::await_or_cancel`
as a separate explicit method for the rare case where the user
wants the framework error union — same shape as GPUI's two-method
split.

**Why polymorphic and not enforce `Task<()>`:** authors that need
the value back imperatively (`let pixels = cx.spawn(load_image).
await;`) lose all ergonomics with `Task<()>`. The polymorphism cost
is one type parameter and one oneshot channel; the ergonomic win
is substantial.

### 7. Backpressure — unbounded queue with debug warnings at 100 in-flight; W3.3-impl writes the bound when load justifies it

Today `sigil-async`'s MPSC is **unbounded** (`tokio::sync::mpsc::
unbounded_channel`). DR-007 §Consequences explicitly accepts this
("we'll bound it once real workloads show a fan-out regression").
The MPSC is per-app; G5 does not change its bound.

For Tasks specifically, G5 adds two debug-time signals:

- **Per-runtime soft warning at > 100 in-flight tasks.** The
  `owner_tasks` registry (DR-007 internal) gains a debug-only check:
  if the per-runtime total exceeds 100, log a `tracing::warn!` once
  per crossing with the spawning entity name + caller `Location`.
  100 is the indie-tier rough cap matching "a button per frame
  spawning a 1s task" worst case; the threshold is a tuning knob.
- **Per-runtime hard panic at > 4096 in-flight tasks.** In debug
  only (release is a no-op). This is the runaway-leak detector. A
  real consumer that legitimately spawns > 4096 tasks (a high-
  throughput agent loop in AI2) configures the cap via
  `AsyncRuntime::set_in_flight_cap(usize)`. Default cap is opt-out,
  not opt-in.

**Why not bounded MPSC + back-pressure on `spawn`:** GPUI is
unbounded and works at IDE scale. Bounding the MPSC would make
`cx.spawn(...)` fallible at the call site, infecting every call site
with `Result<Task<T>, Spawned>` plumbing. Until a real workload
trips on unboundedness, the queue stays unbounded with debug-time
visibility.

**Why not drop-oldest:** Tasks are not interchangeable. Dropping the
oldest is silently losing work the user asked for. Either we
back-pressure (refuse the new spawn) or we accept all spawns; we
choose accept-all-with-visibility, matching DR-007's invariant (5).

### 8. Test seam — `Runtime::run_until_idle()`; pairs with `Scheduler::drain`

```rust
impl Runtime {
    /// Drive the runtime until no work is pending: drain the MPSC,
    /// pump deferred completions, run the observer/effect fan-out,
    /// repeat until quiescent. Blocks the current thread.
    ///
    /// Test-only seam — production frame drivers use
    /// `begin_effect` / `begin_layout` / `begin_paint` / `end_frame`
    /// in sequence (DR-047 §3).
    pub fn run_until_idle(scheduler: &mut dyn Scheduler<...>);
}
```

The signature takes a `&mut Scheduler` because the MPSC drain is
the scheduler's responsibility. `run_until_idle` loops:

```
loop {
    let drained = scheduler.drain();   // MPSC → pending_completions
    Runtime::begin_effect();           // pumps pending_completions + observers
    if drained == 0 && nothing_was_enqueued_during_effect { break }
}
```

Fixed iteration cap (`MAX_QUIESCENCE_PASSES = 64`) catches a
livelock where each completion enqueues another. Cap value matches
DR-047 §8 conservatism.

`run_until_idle` is the only blocking operation in the runtime
surface. Production code never calls it (frames are non-blocking).
Tests use it to await a `cx.spawn(...)`'s completion deterministically
without `std::thread::sleep`.

Sibling API for synchronous-only tests (no live tokio runtime):

```rust
impl Runtime {
    /// Drain just the pending_completions queue and run observers.
    /// Does NOT drive a Tokio runtime; assumes completions were
    /// enqueued by direct test fixture calls.
    pub fn pump_completions();
}
```

`pump_completions` is the lighter version for unit tests that
inject completions without spinning a Tokio worker. Both seams
are documented `#[doc(hidden)]`-test-but-public.

**Why not `block_on(task)`:** blocking on a single Task does not
drive the runtime as a whole. If `task`'s future spawns a peer
task, the peer's completion lands in `pending_completions` but
nothing is calling `begin_effect` to drain it. `run_until_idle`
drives the whole reactor; `block_on(task)` would drive one task at
a time and deadlock on cross-task waits.

### 9. Phase interaction — Paint-phase `cx.spawn` is legal; completion handler is **always** Effect

Three sub-rules cover the DR-047 interaction:

1. **`cx.spawn` itself is legal in every non-Paint phase.** Spawning
   is a registration write: it allocates a tokio task and pushes the
   `AbortHandle` into `owner_tasks`. It does not write to any
   `Entity`, `Signal`, or focus state. Therefore it does not call
   `assert_paint_safe` and does not appear in DR-047 §2's guarded-
   write list. Authors may freely call `cx.spawn` from action
   handlers (Effect), observer callbacks (Effect), or anywhere else
   non-Paint.

2. **`cx.spawn` during Paint is permitted but flagged with a
   debug-only `tracing::debug!`.** A Paint-phase spawn is unusual
   (paint code generally does not initiate background work) but
   not structurally illegal — the spawn registers a task and
   returns immediately. The task's completion still lands in
   `pending_completions` and drains in the next `begin_effect`.
   The debug-trace is the author signal that they are doing
   something unusual; release builds are silent. **This is the
   single point of asymmetry with DR-047 §5's
   `Context::dispatch_action`-during-Paint, which defers; spawn
   does not need to defer because the spawn side-effects are
   already in the right place.**

3. **Completion handlers always run in Effect — never Paint.** This
   is the DR-047 §7 contract. The `pending_completions` drain is
   at the top of `begin_effect`. A future that resolves while the
   main thread is mid-Paint causes the completion to sit in the
   MPSC (and then in `pending_completions` once the next drain
   moves it). The completion fires at the next `begin_effect`. If
   the consumer's frame driver runs `begin_effect → begin_layout →
   begin_paint → end_frame` per frame, the completion runs at most
   one frame after the future resolved. That latency is
   indistinguishable from "the future resolved between frames"
   and is the GPUI shape.

   **Synchronous instant-resolve futures.** `cx.spawn(async { 42 })`
   resolves on the first poll. Tokio's `spawn` still defers to a
   worker, so even instant-resolve goes through the MPSC + drain
   cycle. The completion *cannot* land synchronously inside the
   spawn call, which means the completion *cannot* run in Paint
   even by accident. This is the structural enforcement of the
   §7 contract — there is no synchronous escape hatch in the spawn
   plumbing, by design.

   If a future-resolve happens to land between the MPSC drain and
   `begin_effect` (same-frame turnaround), the completion runs in
   that `begin_effect`. The "always run in Effect" invariant is
   architecturally enforced by the drain seam, not by per-task
   checks.

---

## Implementation seams

- **No new crate.** G5's surface lives in `sigil-reactive`
  (`Context::spawn`, `AsyncContext`, `Task<T>`, `pending_completions`)
  and `sigil-async` (existing `AsyncRuntime` + `Dispatcher` reused;
  one new helper `spawn_task_pair` that returns `(JoinHandle, oneshot::
  Receiver<T>)`). Three files change in reactive, one in async.
- **`crates/reactive/src/task.rs`** (new) — `Task<T>`,
  `TaskAwaitError`, `AsyncContext<U>`, `EntityGone`. ~250 LoC budget.
- **`crates/reactive/src/runtime.rs`** — `RuntimeState` gains one
  field: `pending_completions: Vec<PendingCompletion>`. Initialised
  empty in the existing `const fn new`. `Runtime::begin_effect`
  gains the drain loop at its top.
- **`crates/reactive/src/cx.rs`** — `Context<'_, T>` gains
  `spawn<F, T>(...)` method, ~25 LoC. Plus the
  `AsyncContext::update` round-trip helper (boxed closure → MPSC →
  drain on main thread → oneshot back to future).
- **`crates/reactive/src/lib.rs`** — `pub use task::{Task,
  TaskAwaitError, AsyncContext, EntityGone}`.
- **`crates/async/src/runtime.rs`** —
  `AsyncRuntime::set_in_flight_cap(usize)` plus the debug-time soft/
  hard warn checks in the existing spawn path. Minimal change to
  the public surface.
- **`crates/async/src/dispatch.rs`** — no changes to
  `dispatch_to_owner` / `spawn_to_signal`. `Task<T>` uses the same
  `Dispatcher::send` + `with_owner_by_id` pattern, not a new
  cross-thread plumbing layer.
- **`crates/reactive/tests/task.rs`** (new) — phase contract +
  cancellation + entity-gone tests (test outline in API sketch).
- **`crates/reactive/tests/task_compile_fail.rs`** (new,
  `compile_fail!` doctest equivalent) — verify
  `AsyncContext<U>: Send + Sync` and verify that holding a
  `Context<'_, U>` across `.await` does not compile.

---

## Consequences

- **Two task surfaces coexist.** `spawn_to_signal` →
  `Signal<Status<T, E>>` (reactive-stream shape) and `Context::spawn`
  → `Task<T>` (imperative-handle shape). Both are legitimate; both
  ship; authors pick by use case. They share the underlying
  `AsyncRuntime` + `Dispatcher` plumbing.
- **One new `RuntimeState` field (`pending_completions`).** Drained
  at top of `Runtime::begin_effect`. Pattern parallels DR-047's
  `deferred_writes`.
- **No executor change, no `Cargo.toml` churn.** Tokio is already
  the workspace dep. `sigil-async` already imports the right
  symbols. G5 is a layer atop the existing surface.
- **Cancellation is three-layered and converges.** Explicit drop,
  `.detach()` opt-out, owner cascade. Idempotent at the tokio
  layer. Author intent is honoured at all three.
- **Phase contract is structural, not asserted.** Completions
  always run in Effect because the drain is at `begin_effect`;
  there is no path for a completion to fire elsewhere. Authors
  cannot accidentally violate the §7 contract because there is no
  synchronous-resolve escape hatch.
- **Test seam (`run_until_idle` + `pump_completions`) is
  documented.** Tests no longer need `tokio::time::sleep`-then-
  pump; they drive to quiescence deterministically.
- **AI-track agent-loop primitives (AI2) build on this.** Every
  tool call is a `cx.spawn(|acx| async move { ... acx.update(...) })`.
  The tool-call lifecycle (request → tool → result-write-back) is
  exactly one `Task<ToolResult>`.
- **the host engine live-reload (FX4) builds on this.** A live-reload
  detector spawns a watcher Task; on change, the completion
  enters Effect, dispatches a `ReloadComponent` action, and the
  reactive cascade rewires the component graph without crossing
  the type system at the seam.
- **Bench surface lands in W3.3-impl.** Criterion benches in
  `crates/reactive/benches/task.rs`: `spawn_immediate_resolve`
  (target ≤ 5 µs end-to-end), `spawn_50_concurrent_completions`
  (target ≤ 200 µs total drain), `update_round_trip`
  (target ≤ 50 µs main-thread → worker → main-thread). Numbers
  are first-draft; W3.3-impl re-fits on real measurements.
- **`Subscription` leak class story extends.** Tasks share the
  same RAII-or-owner-cascade safety net. Forgotten handles are
  cleaned up by owner disposal at the latest, by `Drop` at the
  earliest.

---

## What this does NOT cover

Explicit scope cuts.

- **Multi-thread background task pool tuning.** G5 uses Tokio's
  default thread pool; sizing / pinning / priority is DR-007's
  territory.
- **Cross-runtime task migration.** A task spawned on runtime A
  cannot await on runtime B. Sigil has one runtime per app; this
  is not a real constraint.
- **WASM-target compatibility.** Out of scope per DR-040. If sigil
  ever ships a WASM backend, the `Send`-bound on futures becomes
  the seam (WASM is single-threaded; the executor changes; the
  `Send` bound becomes vacuous).
- **`Task<T>::poll` from a non-runtime thread.** `Task<T>: Future`
  but the Future is polled on the main thread (where the
  `Context` was created). Polling from a tokio worker is
  technically possible but unsupported.
- **Structured concurrency primitives** (`select!`, `join_all`,
  `try_join_all`). Available via the existing tokio crate;
  G5 does not re-export. Authors `use tokio::{select, join}`
  directly.
- **Cancellation-aware future utilities** (cooperative
  `CancellationToken`). Tokio's `CancellationToken` works
  unchanged inside `cx.spawn` futures. Sigil does not ship a
  parallel cancellation primitive.
- **Async iterators / streams returned from `cx.spawn`.** A future
  resolves once; a stream emits many times. Streams that drive
  reactive state should use `spawn_to_signal`-shape signals or
  push values via `AsyncContext::update` from inside the stream
  consumer loop.
- **Task priorities.** Tokio does not natively support task
  priorities; we do not synthesise one.
- **Task naming / debugger introspection.** A `name: &'static str`
  field on `Task<T>` and a runtime registry of named tasks is a
  reasonable follow-up (parallels `Entity::name`). Deferred to
  W3.3-impl implementation discovery.
- **Multi-window per-window task tagging.** Sigil is single-
  window today (per DR-047 §8 open question 8). Multi-window
  tagging is a forward concern.

---

## Alternatives considered

### Executor: smol / async-executor

Rejected — DR-007 considered both and accepted Tokio for proven-at-
scale and winit-integration reasons. Re-deciding executors in G5
would invalidate `sigil-async`'s adapter, `Dispatcher`, every
`OwnerHandle` semantic, and the `spawn_to_signal` test surface for
no G5-specific win. Locked at DR-007; G5 inherits.

### Executor: single-threaded `LocalSet` so futures can hold `Entity<T>`

Rejected. `LocalSet` makes the future `!Send`, which excludes
every `Send`-bounded async library (`reqwest`, `tokio::fs`,
`hyper`, every published HTTP/grpc/DB client). The trade —
"borrow Entity directly *or* use any existing async library, pick
one" — is the worst-of-both. GPUI made this trade explicitly with
`Send`-bounded futures + `cx.update` trampoline and shipped at
Zed-scale; we mirror.

### Futures are `!Send`, runtime adds a `current_thread` Tokio backend

Rejected. Same fault as `LocalSet`. Single-threaded Tokio is still
single-threaded; ecosystem libraries assume multi-thread spawn.
Adding a per-app Tokio backend toggle bifurcates the user-facing
contract.

### `Task<T>` is just a future-handle (no Drop-cancel)

Rejected. Forgotten handles leak background tasks until owner
disposal eventually cancels them. The owner-cascade is the safety
net, not the primary cancellation path; explicit cancellation at
Drop is the per-task contract. Default-drop-cancels matches GPUI
and matches author intent ("I dropped this; I'm done with it").

### Cancellation via explicit `.cancel()` only

Rejected. The forgetful-author cost (one leaked task per scope) is
worse than the rare-fire-and-forget cost (one `.detach()` call).
Drop-cancel by default; opt out by name.

### `Task<T>` combines Drop-cancel and Drop-detach into one no-op Drop

Rejected. Same as above plus a documentation-only signal — Authors
who don't read the docs leak. The `#[must_use]` warning catches
"you ignored the handle"; the Drop catches "you intentionally let
the handle go out of scope without detaching."

### Completion delivered immediately (not deferred)

Rejected. Violates DR-047 §7. Synchronous completion of an instant-
resolve future would let the completion run in any phase the spawn
was issued in — including Paint. The drain-at-`begin_effect`
seam structurally prevents this.

### `pending_completions` reuses `deferred_writes`
(DR-047 §5)

Rejected. The two queues drain at different seams (`begin_effect`
vs `end_frame`) and carry different payloads (raw closures vs
`Box<dyn Action>`). Combining them requires a per-entry phase tag
and a two-pass drain at every transition. Splitting them keeps
each queue's contract crisp and grep-able.

### `AsyncContext<U>` exposes `&mut Entity<U>` directly across await

Rejected. `Entity<U>` is Copy and Send-compatible at the *handle*
level, but mutating the entity requires `with_rt` borrowing the
runtime, which is `!Send`. Crossing the await would require
holding the `with_rt` borrow across the await point — which is the
exact re-entrant-`RefCell` deadlock the crate is structured to
avoid. The `update`-closure trampoline is the only correct shape.

### `AsyncContext::update` returns `R` directly (not `Result<R,
EntityGone>`)

Rejected. The entity may have been disposed between spawn and
update. Panicking is wrong (background tasks don't get to crash the
process); returning `Result` lets the user choose between
`.expect("entity still alive")` (panic-equivalent) and `if let Ok`
(graceful). Fails-soft is the documented `OwnerHandle::is_alive`
shape from DR-007.

### Task body receives `Context<'_, U>` via a custom `LocalScope` async runtime

Rejected. Would require a custom executor that knows how to suspend
across `with_rt` borrows. The complexity is multi-month and the win
is "the author saves typing `acx.update(|this, cx| ...)`." Not
worth.

### Bound the `Dispatcher` MPSC at fixed capacity

Rejected. DR-007 invariant (5) defers this to real workloads.
G5 does not change the queue size; it adds debug visibility (soft
warn at 100, hard panic at 4096) at the spawn site so leaks
surface fast. Real bound lands when a real workload trips on
unbounded growth.

### Drop-oldest backpressure on `cx.spawn`

Rejected. Tasks are not interchangeable. Silently dropping one is
losing work the user asked for, with no way to detect it. Accept-
all-with-visibility is the indie-tier-correct shape.

### `Task<T>` is `!Send` even though future is `Send`

Rejected. Authors that hold a Task across boundaries (a state
field on `Entity<U>` is fine — `Entity` is `!Send`, the Task lives
on the main thread alongside it) don't need `Task: Send`; the
Task is created on the main thread, polled on the main thread,
dropped on the main thread. Marking `Task: !Send` is the boring-
correct choice that matches the runtime's overall main-thread
discipline. We make this explicit via `PhantomData<*const ()>`.

### Test seam: `block_on(task)`

Rejected. `block_on` drives a single future, not the whole
runtime. A task that spawns peer tasks deadlocks because nothing
calls `begin_effect` between waits. `run_until_idle` drives the
whole reactor including peer completions.

### Combine `pump_completions` with `Runtime::flush()`

Rejected — `flush` is a name the project has not used; introducing
it conflates "drive the runtime" with "synchronise output." Two
small named seams (`run_until_idle` for the with-tokio case,
`pump_completions` for the without-tokio case) is clearer than
one overloaded `flush`.

---

## Open questions

These are explicitly **not** resolved in this DR. They land in
W3.3-impl implementation discovery or a follow-up DR.

1. **`Task::abort_on_drop` opt-out at type level.** Today the user
   calls `.detach()` to opt out of Drop-cancel. An alternative is
   a type-level flag (`Task<T, AbortOnDrop>` vs `Task<T, NoAbort>`)
   that lets the compiler reject mistakes earlier. Provisional:
   stay runtime-only for symmetry with `Subscription`. Re-evaluate
   if real consumers want compile-time enforcement.

2. **`Task<T>: Future<Output = T>` vs `Future<Output = Result<T,
   TaskAwaitError>>`.** Decision 6 says the common case is the
   former; the explicit-error case is `Task::await_or_cancel`. The
   exact split depends on consumer ergonomics across the AI2
   agent loop. W3.3-impl validates against a real consumer (the
   first AI-track agent loop call) and locks the split before
   shipping the public API.

3. **Per-Task naming for debugger / tracing visibility.** Mirrors
   `Entity::name`. Provisional shape:
   `cx.spawn_named("fetch_diagnostics", |acx| async move { ... })`.
   Deferred; the non-named `cx.spawn` is the load-bearing surface.

4. **`AsyncContext` extension for cross-entity observation.** Today
   the only way to react to an entity from a task is to poll via
   `acx.update_entity(entity, |this, cx| read).await`. A future
   `acx.observe_entity(entity).next().await`-style stream is a
   reasonable ergonomic add but is not the load-bearing surface.
   Deferred until a real consumer needs it.

5. **`run_until_idle` quiescence-cap value.** Decision 8 says 64;
   the value is conservative and may be too low for some legitimate
   cascade depths. W3.3-impl picks a final number based on the
   AI2 agent-loop fixture and the deepest observed completion-of-
   completion chain.

6. **Multi-window task tagging.** Sigil is single-window today.
   Multi-window (Phase X) will need per-window task tags so
   `run_until_idle` on window A doesn't drain completions for
   window B. Out of scope for G5; documented as a forward concern.

7. **`AsyncContext::update` reentrancy.** A handler called from
   `update` may itself await another `update`. The two updates
   queue on the MPSC; no deadlock by construction (the inner
   update's main-thread closure runs at a later drain, not
   synchronously). Edge: the outer update's closure body cannot
   await — it's a synchronous `FnOnce`. Authors await
   *outside* the update closure. This is the GPUI shape; document
   it in the API sketch.

8. **`Task::then` / chained spawn.** A future-on-task composition
   is achievable via `cx.spawn(|acx| async move { let a =
   spawn_a(acx).await; let b = spawn_b(acx, a).await; b })`. We
   do not add a `Task::then` combinator; users compose via
   plain `async/await`. Re-evaluate if a real consumer asks.

---

## References

- **DR-040** — sigil thesis lock (own the foundation; mirror GPUI).
- **DR-045** — G2 entity model (`Entity<T>` + `Context<'_, T>` —
  the re-entry target for `AsyncContext::update`).
- **DR-046** — G3 actions + focus (action handlers spawn Tasks;
  open question 6 cited this DR for `dispatch_action_async`).
- **DR-047** — G4 DrawPhase (§7 pre-locked completion-runs-in-
  Effect; §5 deferred-write-queue is the pattern
  `pending_completions` parallels).
- **DR-007** — one Tokio runtime per app (the executor G5 sits
  on; G5 does not revisit).
- **DR-005** — reactive owner tree (the cancellation spine Tasks
  parent against; `with_owner_by_id` is the trampoline).
- **Master dispatch plan** —
  `docs/plans/2026-05-18-parallel-dispatch-master.md` (W3.3 is
  this DR; W3.3-impl is the implementation).
- **API sketch** — `docs/specs/g5-task-api-sketch.md` (companion
  to this DR).
- **`crates/async/src/runtime.rs`** — `AsyncRuntime` +
  `Dispatcher` + `Scheduler::spawn`; G5 layers `Task<T>` atop.
- **`crates/async/src/dispatch.rs`** —
  `spawn_to_signal` is the closest existing analog;
  `Context::spawn` reuses its plumbing shape.
- **`crates/reactive/src/cx.rs`** — `Context<'_, T>` shape;
  `spawn` slots in alongside `dispatch_action`.
- **`crates/reactive/src/runtime.rs`** — `RuntimeState`;
  `pending_completions` field lands here, drained at
  `begin_effect` (DR-047 §3).
- **GPUI reference** — `zed-industries/zed/crates/gpui`
  (`Task<T>`, `AsyncWindowContext`, `AsyncAppContext`,
  `cx.spawn`, `.detach()`).

---

## Amendment 2026-05-18 — `cx.spawn` panics in Paint (drop §9 rule 2)

W3.3-impl surfaced a structural contradiction inside this DR.

Decision 9 rule 2 above declared `cx.spawn` "permitted but flagged with
a debug-only `tracing::debug!`" during Paint, with the argument that
spawn-side-effects do not write to reactive state. But the only public
path to a `Context<'_, T>` is `Entity::update`, and **DR-047 §2 requires
`Entity::update` to panic during Paint** (see
`crates/reactive/src/entity.rs::run_update` line 240 — the very first
thing `run_update` does is `assert_paint_safe`). So `cx.spawn` "during
Paint" is structurally unreachable from any user code path: to even
hold a `Context<'_, T>` during Paint, an author would have to bypass
`Entity::update`, and there is no such bypass.

The §9 rule 2 wording was carried over from an earlier draft when the
phase-asserting seam was being prototyped on `notify()` rather than on
`update`. With DR-047 §2 locked, the spawn-legal-in-Paint claim is
contradictory and confuses downstream contracts (action-handler authors
read §9 and ask "can a Paint observer fire-and-forget?" — the answer is
no, but only because §9's premise is unreachable).

### B1 — `cx.spawn` panics in Paint, matching every other write seam

`Context::spawn` (`crates/reactive/src/cx.rs:109`) joins the guarded-
write list. The first line of the method calls
`crate::phase::assert_paint_safe(rt, "Context::spawn")` inside the
existing `with_rt` borrow, before any executor lookup. Identical shape
to `Context::notify` (cx.rs:74), `Context::emit` (cx.rs:201),
`Context::focus` (cx.rs:342), `Context::blur` (cx.rs:352),
`Context::dispatch_action` (cx.rs:427), and `Entity::update`
(entity.rs:240).

The `tracing::debug!`-during-Paint reference in §9 rule 2 is dropped.
There is no Paint-phase spawn path; there is nothing to trace.

### B2 — §9 rewrite

§9 of the original Decision section is amended to read (load-bearing
text, paraphrased here for the amendment; the body above stays for
historical context):

> **9. Phase interaction.** Three sub-rules cover the DR-047
> interaction.
>
> 1. `cx.spawn` is legal in Idle / Effect / Layout, matching every
>    other reactive write seam (DR-047 §2). The implementation calls
>    `assert_paint_safe("Context::spawn")` as its first action.
> 2. `cx.spawn` **panics in Paint**. The author cannot reach a
>    `Context<'_, T>` from inside Paint anyway (DR-047 §2 prevents
>    `Entity::update` in Paint), so the panic is a defense-in-depth
>    guard, not a behaviour authors will hit at the call site.
>    Authors who want a paint-pass widget to initiate background work
>    must defer through an action or dispatch handler that runs in
>    Effect — the same shape DR-047 §5 prescribes for any other
>    Paint-phase write intent.
> 3. **Completion handlers always run in Effect — never Paint.** No
>    change from the original §9 rule 3. The drain seam at the top of
>    `Runtime::begin_effect` is the structural enforcement; there is
>    no synchronous-resolve escape hatch in the spawn plumbing.
>
> Synchronous instant-resolve futures (`cx.spawn(async { 42 })`) and
> the inter-frame-arrival case described in the original §9 rule 3
> are unchanged.

The "single point of asymmetry with `Context::dispatch_action`-during-
Paint" note in the original rule 2 is **removed** — there is no
asymmetry now. `dispatch_action` defers; `spawn` panics; both are
consistent with the rest of the write-seam family.

### B3 — Implementation impact

The amendment is a contract change. The code change is a one-liner in
`crates/reactive/src/cx.rs::Context::spawn`:

```text
pub fn spawn<F, R>(...) -> Task<R> {
    let entity_key = with_rt(|rt| {
        crate::phase::assert_paint_safe(rt, "Context::spawn");  // NEW
        // ... existing body ...
    });
    // ... existing body ...
}
```

The matching test lands in `crates/reactive/tests/task.rs` (or
extends the phase-contract suite already there):

```text
#[test]
#[should_panic(expected = "Context::spawn")]
fn spawn_in_paint_panics() {
    // Build runtime + entity, enter Paint phase via the existing test
    // helper, attempt cx.spawn inside an entity.update equivalent
    // (reach by direct phase manipulation, mirroring the existing
    // dispatch_action_in_paint_defers test fixture).
}
```

Note: the test cannot reach `cx.spawn` through `entity.update` because
`update` panics first. The test must use the same fixture pattern the
DR-047 phase tests use to drive `Context` while phase is Paint (see
the existing dispatch-action paint test for the pattern).

### Option B rejected — phase-bypassing `PaintContext<T>` for spawn

An alternative considered: introduce a `PaintContext<'a, T>` exposed
during the paint pass that allows `cx.spawn` but not `cx.update`. The
spawn registers a future without mutating reactive state, so it is
safe in Paint by construction.

Rejected for three reasons:

1. **No real consumer asks for it.** No widget reviewed under DR-047
   needs to initiate background work during paint; the natural seam
   is the action that triggered the paint, which runs in Effect.
2. **Two Context flavours doubles the API surface.** Every helper
   currently on `Context<'_, T>` (`notify`, `observe`, `subscribe`,
   `focus`, `dispatch_action`, `spawn`) has to be triaged for paint-
   legality, and each gets a parallel `PaintContext` slot or doesn't.
   The triage matrix is more architectural surface than the win
   justifies.
3. **`Context::spawn`-panics is the boring-consistent shape.** Every
   reactive write seam panics in Paint; spawning a task to mutate
   reactive state later is morally a write; the consistent rule is
   "Paint is the no-side-effects phase." Adding an exception for
   spawn weakens that mental model for no concrete win.

Option A (this amendment) is the chosen resolution.

### Downstream implementation tasks

Queued as follow-ups for the next reactive-implementer dispatch:

- `crates/reactive/src/cx.rs` — add `assert_paint_safe("Context::spawn")`
  as the first action inside `Context::spawn`'s `with_rt` block.
- `crates/reactive/tests/task.rs` — `spawn_in_paint_panics` test plus
  any nearby fixture sharing for phase manipulation.
- Documentation: the `Context::spawn` doc comment currently references
  DR-048 §9; it should be updated post-amendment to read "panics if
  called during Paint" alongside the existing executor-required note.

