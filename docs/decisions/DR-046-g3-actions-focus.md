---
id: DR-046
status: proposed
date: 2026-05-18
covers: G3 typed action dispatch + focus-context resolution (W3.1a design, W3.1b implementation)
depends_on: [DR-040, DR-045, DR-005]
relates: docs/plans/2026-05-16-gpui-parity-integration.md, docs/specs/g3-actions-api-sketch.md
---

# DR-046 — G3 Typed Action Dispatch + Focus-Context Resolution

**Status:** Proposed
**Date:** 2026-05-18
**Covers:** G3 keystone — typed `Action` trait + `#[derive(Action)]`, `FocusHandle`,
`cx.on_action`, `cx.dispatch_action`, `cx.propagate`, `cx.focus`,
`cx.is_focused`, most-nested-override resolution, opt-in propagation.
**Depends on:** DR-040 (sigil thesis lock — "mirror GPUI's validated shape"),
DR-045 (`Entity<T>` + `Context<'_, T>` — actions hang off entities and
their reactive owners), DR-005 (owner tree — focus handles are
owner-scoped and dispose with their owner).
**Unlocks:** G4 DrawPhase (action handlers execute in the Update phase,
not Paint — assert seam citation), G5 `Task<T>` (long-running action
handlers spawn entity-owned tasks), Phase I/AI palette + keymap layers
(I6 keymap is the consumer of the dispatch surface defined here; AI6
AI-routed palette actions are the consumer of the same registry).

---

## Context

DR-040 commits sigil to mirroring GPUI's validated framework substrate.
DR-045 landed `Entity<T>` and `Context<'_, T>` — the borrow handed to
update closures and observer callbacks. G3 is the next substrate piece:
**a typed action dispatch model with focus-context override resolution.**

GPUI's action model is the load-bearing piece of Zed's authoring story:

```rust
actions!(workspace, [CloseActiveItem, SaveAll, FollowUp]);

// Register a handler on an entity. The handler is scoped to that
// entity's focus context — the action only fires when focus is within
// the entity's subtree.
cx.on_action(|this: &mut Workspace, _: &SaveAll, cx: &mut Context<_>| {
    this.save_all(cx);
});

// Dispatch fires the most-nested handler in the focused entity's
// ancestry. Outer handlers receive the action only if inner handlers
// call cx.propagate().
cx.dispatch_action(&SaveAll);
```

Two non-obvious properties make this work for an IDE-class consumer:

1. **Type-keyed registration.** Actions are types, not strings.
   `cx.on_action::<SaveAll>` registers against `TypeId::of::<SaveAll>()`.
   This is the seam that lets a third-party plugin declare its own
   action type and route it through the same dispatch path without
   string-key collisions.
2. **Focus-context resolution.** A handler registered on entity *A* only
   fires when the focused entity is *A* or a descendant of *A*. The
   most-nested matching handler in the focus path wins; outer handlers
   are silent unless that handler calls `cx.propagate()`.

What already exists in this repo:

- **`sigil-reactive`** ships `Entity<T>`, `Context<'_, T>`,
  `Subscription`, observer fan-out, and owner-cascade disposal
  (DR-045). Action handlers will hang off `Entity<T>` exactly the way
  observers do.
- **`sigil-events`** already routes pointer/key events through the
  composed node tree, maintains a `FocusManager` keyed by
  `sigil_kernel::NodeId` (the visual node), and dispatches
  semantic-`Shortcut` matches via a `ShortcutIndex` that is rebuilt on
  every `Cx::mutation_generation()` bump (see
  `crates/events/src/router.rs` lines 273–315).
- **`sigil-kernel`** owns the node tree and `Cx` composition context.

What does not exist:

- **No typed action enum.** `sigil-events` dispatches raw
  `KeyEvent`/`PointerEvent` shaped after winit; there is no
  intermediate "action" layer between key→event and widget→behavior.
- **No focus on entities.** `FocusManager` focuses *nodes*
  (`sigil_kernel::NodeId`), not entities. GPUI's model is focus *on
  entities*; without a translation seam the action override resolution
  has nothing to walk.
- **No action registry.** Type-keyed handler storage, dispatch
  iteration, propagation control — none of it exists yet.

The mistake to avoid is hacking "shortcut → method on widget" plumbing
into the events crate one widget at a time. That produces an IDE-grade
keymap (I6) that is welded to widget code and unreachable from a
plugin host (I5). G3 owns the indirection layer that lets a plugin
declare a `RefactorRename` action and bind it to a key without forking
the widget that handles renames.

### Reading the existing repo

- `crates/reactive/src/lib.rs` lines 45–79 — module layout and public
  re-exports. The action layer slots in as `crates/reactive/src/action*`
  modules with crate-root re-exports, paralleling the entity layer.
- `crates/reactive/src/cx.rs` lines 39–60 — `Context<'_, T>` is a
  borrow-checked token; it does not hold `&mut RuntimeState`. Methods
  call `with_rt` themselves. Action methods on `Context` follow the
  same pattern.
- `crates/events/src/focus.rs` lines 10–82 — `FocusManager` is the
  existing focus state. It is node-keyed; G3 adds an entity-keyed layer
  on top, and a bridge from focused-`NodeId` ↔ focused-`Entity<_>` via
  a node-side `FocusHandle` extension.
- `crates/events/src/router.rs` lines 217–239 — key dispatch path. G3
  intercepts here: after `ShortcutIndex::dispatch` and before
  `dispatch_focused_key`, a new `KeymapBindings` layer (deferred to a
  follow-up DR; see Decision 5) translates `KeyEvent` → `Box<dyn Action>`
  and calls `Runtime::dispatch_action` on the result. If no binding
  matches, the existing `dispatch_focused_key` path runs unchanged.

---

## Decision

Seven decisions, each frozen for the W3.1b implementer.

### 1. Action representation — `Action` trait + `#[derive(Action)]`

```rust
// Crate: sigil-reactive (the trait); sigil-derive (the proc-macro).
pub trait Action: Any + Send + Sync + 'static {
    fn name() -> &'static str where Self: Sized;
    fn namespace() -> &'static str where Self: Sized;
    fn boxed_clone(&self) -> Box<dyn Action>;
    fn as_any(&self) -> &dyn Any;
    fn eq_action(&self, other: &dyn Action) -> bool;
}
```

`#[derive(Action)]` generates `name()`, `namespace()` (from a
`#[action(namespace = "workspace")]` attribute), `boxed_clone`,
`as_any`, and `eq_action` (using `PartialEq` on the underlying type).
Macro lives in a new `crates/reactive-derive/` proc-macro crate
(matching the sigil-style `crates/*` layout); the trait lives in
`sigil_reactive::action`. Action *types* are `Send + Sync + 'static`
so they can be serialized into a plugin host or a future
keymap-file format; the *registry* and dispatch are main-thread only.

Why a trait + derive, not a single enum: a global `enum Action {
SaveAll, CloseTab, RefactorRename, … }` forces every action across
every consumer (sigil itself + the host engine editor + every third-party plugin)
into one file. The plugin host (I5) would have no way to declare a new
action without patching `sigil-reactive`. GPUI's `actions!` macro and
sigil's `#[derive(Action)]` solve the same problem by making each
action a discrete type and the dispatch keyed by `TypeId`.

Why not `dyn Action` with manual impl: the boilerplate
(`name()`/`namespace()`/`boxed_clone`/`as_any`/`eq_action`) is exactly
the macro-eligible pattern. Hand-rolling it makes the action surface
visually heavy; consumer code reads `#[derive(Action)] struct
SaveAll;` and that is the entire authoring cost. Plugin authors get
the same one-line ergonomics sigil-internal authors get.

### 2. Action registry — per-entity handler list + runtime aggregate

Storage shape:

- **Per-entity:** each `EntitySlot` gains
  `actions: SmallVec<[ActionHandlerKey; 4]>` — an inline list of
  handler keys this entity has registered.
- **Per-runtime:** a flat `ActionHandlerArena: Arena<ActionHandlerSlot>`
  on `RuntimeState`, paralleling `entities` and `observers`. Each slot
  carries `{ entity: EntityKey, type_id: TypeId, handler: Box<dyn
  ActionHandler>, location: &'static Location }`.

Dispatch path: `Runtime::dispatch_action(action: &dyn Action)` walks
the **focused entity's ancestry** (decision 3 — focus path), and for
each entity in that path iterates its `actions` list searching for a
`type_id == TypeId::of::<A>()` match. The most-nested match fires
first; outer matches fire only if the inner handler called
`cx.propagate()` (decision 6).

Why the split (per-entity list + per-runtime arena), not a global
`HashMap<TypeId, Vec<HandlerKey>>` indexed only by type:

- A single global `HashMap<TypeId, Vec<HandlerKey>>` cannot answer
  "what handlers should fire for this focused entity?" without an
  extra "which entities are in the focus path?" walk and per-entry
  filtering. With per-entity lists, the dispatch walk is one cache-line
  fetch per ancestor: load the entity's tiny inline `SmallVec`, linear
  scan it (≤ 4 entries for typical entities — buttons have 1–2
  actions, workspaces have 8–15).
- Linear scan over ≤ 4 entries beats a hash lookup; the action count
  per entity is bounded by authoring discipline (one entity = one
  cohesive feature surface = few actions). If a real consumer registers
  > 16 actions on one entity, we re-fit; for now `SmallVec<[_; 4]>`
  inlines.
- The per-runtime arena gives us stable `ActionHandlerKey` values that
  the `Subscription`-equivalent (a returned RAII handle from
  `cx.on_action`) drops to deregister, identical to the entity-observer
  story (DR-045 §4).

Entity dispose cascade: when `dispose_owner` walks an entity's
`entities` list, the entity removal pass also drains its `actions` list
and removes each handler from the per-runtime arena. This is the same
free-cleanup pattern observers already get.

### 3. Focus context — `FocusHandle` + focus stack on `RuntimeState`

A `FocusHandle` is a typed token that an entity holds to declare it can
receive focus:

```rust
pub struct FocusHandle {
    key: FocusKey,        // generational; like SignalKey / EntityKey.
    _not_send: PhantomData<*const ()>,
}
```

`FocusHandle` is created via `cx.focus_handle()` inside an
`Entity::update` closure. The handle is *bound to* the surrounding
entity — `cx.focus_handle()` records the binding
`{ focus_key → entity_key, owner_key }` in
`RuntimeState::focus_bindings`. The handle is `!Send + !Sync` and lives
on its owning entity's `FocusHandle`-typed field (typically as a
`Subscription`-style RAII member).

`RuntimeState` gains:

- `focus_arena: Arena<FocusSlot>` — slots carry `{ entity: EntityKey,
  owner: OwnerKey, parent_focus: Option<FocusKey> }`.
- `focused: Option<FocusKey>` — the single currently-focused handle.
- `focus_bindings: AHashMap<FocusKey, EntityKey>` — looked up by
  dispatch to map focus → owning entity.

Focus *path* is computed by walking from the focused entity up through
the **reactive owner ancestry** (DR-005 owner tree), collecting every
ancestor entity that has at least one registered action handler. This
is the GPUI shape: focus context = focused entity + every entity in
the focus path that has registered handlers. Outer entities along the
owner chain are the targets the action can bubble to.

We use *owner ancestry*, not *node-tree ancestry* (`sigil-kernel`), as
the spine for focus path resolution. Rationale: entities are owned by
reactive owners (DR-045); the owner tree IS the entity tree. The node
tree (visual scene-graph) is one of many consumers of entities, not
the source of truth for entity hierarchy. Coupling action dispatch to
the node tree would force every action consumer to also have a node,
which AI agent-loop tools (no node), background tasks (no node), and
headless integration tests (no node tree at all) would all fail.

Why not "walk-the-kernel-tree from focused node":

- The kernel tree is the *visual* tree. Two different entities may
  share a single composite node, or one entity may own multiple
  nodes — the mapping is many-to-many. Walking the visual tree to find
  action handlers requires every entity to register against a node,
  which contradicts the entity model.
- The reactive owner tree already encodes the parent-child
  relationship action dispatch needs. The cost of walking it is the
  same as walking the node tree; the correctness is strictly better.

Why not "reactive `Signal<Option<Entity>>` for focus":

- A signal-driven focus would re-fire effects on every focus change
  — every observer of the focus signal would wake on every keystroke
  that moves focus. Focus changes are not data the application needs
  to react to as a stream; they are state the dispatch resolver needs
  to read once per dispatch.
- A single `Option<FocusKey>` on `RuntimeState` is enough. Widgets
  that need to react to focus (`on_focus_in`/`on_focus_out`) get the
  notification via existing handler invocation (the events-crate
  `FocusManager` fires those callbacks already; we keep that wiring
  unchanged).

### 4. Override resolution — most-nested wins; outer entities are bubble targets

Dispatch walks the focused entity's owner ancestry **leaf-first**
(focused entity itself first, then its parent entity, then its
grandparent, …). At each step:

1. Read the entity's `actions` list.
2. Iterate for a `type_id` match against the dispatched action's
   `TypeId`.
3. If found: invoke the handler with `(&mut This, &A, &mut
   Context<This>)` and *stop*, unless the handler called
   `cx.propagate()` (decision 6).
4. If not found: continue to the next ancestor.

If no handler matches anywhere in the path, the action is dropped
silently (this is GPUI's behavior — undeclared actions are no-ops, not
errors). A *debug-only* trace log records the unhandled dispatch; the
release build is silent.

Override syntax — the most-nested entity in the focus path that has a
handler for action `A` wins. Authors do not declare "I override the
outer handler"; they simply register a handler on a more-nested
entity, and dispatch sees their handler first. To declare "I handle
this action but also want the outer handler to run," the handler
explicitly calls `cx.propagate()`.

This shape is the GPUI shape and is the only override resolution
strategy this DR endorses. The alternatives considered (priority
attributes, registration-order, scope-stack-explicit overrides) are
rejected — see Alternatives.

### 5. Keyboard binding — keymap layer is separate, deferred to DR-049

Keymap bindings (key → action lookup, key sequence resolution, modal
keymap layers) are **not** in G3. G3 lands the action layer; bindings
are a layer above it.

Concretely:

- `sigil-reactive::action::Runtime::dispatch_action(action: &dyn
  Action)` is the entry point. Callers translate input into actions
  themselves.
- The events crate already dispatches winit `KeyEvent` and
  shortcut-matched semantic events through its existing path.
- A future **DR-049 / I6 (Keymap engine)** introduces a `Keymap` type
  on top of `Runtime::dispatch_action` that owns the
  `KeySequence → Box<dyn Action>` resolution table, the modal layer
  stack, the timeout-resolved-as-sequence policy. The keymap consumes
  G3 — G3 does not consume the keymap.

Actions do **not** self-declare their bindings via attribute
(`#[action(key = "ctrl-s")]` rejected). Bindings are user-configurable
(per the IDE story); baking them into the type means a plugin author
fixes a binding that the consumer expected to remap. The keymap-DSL
choice (TOML vs JSON vs Rust-DSL vs query language) is the subject of
DR-049 and is explicitly deferred here.

The seam G3 exposes to the keymap layer is one function:

```rust
pub fn dispatch_action(action: &dyn Action);
```

Anything that produces a `&dyn Action` can call this — a keymap, a
palette, an AI agent, a test harness. This is the indirection that
makes I5 (plugin host) and AI6 (AI-routed palette) consume the same
substrate.

### 6. Propagation — implicit stop with explicit `cx.propagate()`

Dispatch stops at the first handler match (decision 4). To let outer
handlers run, the inner handler calls:

```rust
cx.propagate();
```

`cx.propagate()` is a flag on the active dispatch frame stored in
`RuntimeState::current_dispatch: Option<DispatchFrame>`. The
`DispatchFrame` carries `{ action_type: TypeId, propagate: bool }`.
When the handler returns, the dispatcher checks `propagate`; if true,
it continues up the focus path to the next ancestor. If false (the
default), the dispatch ends after this handler.

This matches GPUI exactly. The reasoning is the same:

- Most actions want stop-at-first-match (a "close tab" action handled
  by the tab widget should not also fire the workspace's "close pane"
  action with the same name from a parent context).
- The cases that *do* want bubble (e.g. an outer "save scroll position"
  handler observing every action) are rare enough that they should be
  explicit. The default is "I handle this; nobody else needs to know."

`cx.propagate()` is callable from within `cx.on_action` handler bodies
only. Calling it outside an active dispatch is a debug-assert panic
("no dispatch frame to propagate").

### 7. Pointer-event interop — `sigil-events` and the action layer stay separate

`sigil-events` (PointerEvent / FocusEvent / KeyEvent / Shortcut)
remains the input-routing layer. G3 does **not** absorb pointer
routing. The action layer is invoked from above the events layer (by
keymaps, palettes, AI agents) and from inside event handler bodies
(`on_click` of a button calls `cx.dispatch_action(&SaveAll)`).

Concretely:

- `sigil-events::FocusManager` keeps focusing `sigil_kernel::NodeId`
  values. The node-side `FocusHandle` extension (a small kernel
  `Extension`) carries `{ focus_key: FocusKey, … }` so that the
  events crate can call `Runtime::set_focus(focus_key)` when the
  node-focus changes. This is the single bridge between node-focus
  and entity-focus.
- `sigil-events::EventRouter::route_key` continues to handle Tab
  cycling, shortcut dispatch, and `dispatch_focused_key` exactly as
  it does today. The keymap layer (DR-049) inserts a single hook in
  `route_key` *before* `dispatch_focused_key`: it consults the
  keymap, and on a match, calls `Runtime::dispatch_action`. On no
  match, the existing path runs unchanged.

This separation matches the seam between
`gpui::EventEmitter` (pointer/key event delivery) and the action
layer in Zed. The two responsibilities — input routing vs typed
dispatch — are orthogonal, and merging them would mean one bug class
in the input layer breaks every plugin's action handlers.

---

## Consequences

- **One new crate (`sigil-reactive-derive`)** for the `#[derive(Action)]`
  proc-macro. Lives at `crates/reactive/reactive-derive/` (path-nested
  per the project's crate-naming convention). `sigil-reactive` takes
  it as an optional dependency behind an `actions-derive` feature
  flag, off by default for downstream consumers that don't need
  proc-macros. The trait `Action` is always available; the derive is
  the ergonomic shortcut.
- **`crates/reactive/src/` gains four new modules** (~250 LoC budget
  each, hard-split if any exceeds 350): `action.rs` (the `Action`
  trait + `Box<dyn Action>` plumbing), `action_handler.rs`
  (`ActionHandlerArena`, registration, dispatch loop), `focus.rs`
  (`FocusHandle`, `FocusKey`, `focus_arena`, focus path resolver),
  and an extension of `cx.rs` for the new Context methods. Public
  re-exports in `lib.rs`: `pub use action::Action; pub use focus::{
  FocusHandle, FocusKey };`.
- **`RuntimeState` gains five fields:** `actions:
  Arena<ActionHandlerSlot>`, `focus_arena: Arena<FocusSlot>`,
  `focused: Option<FocusKey>`, `focus_bindings: AHashMap<FocusKey,
  EntityKey>`, `current_dispatch: Option<DispatchFrame>`. Each is
  initialized empty; the existing `RuntimeState::new` path adds five
  one-line constructor calls.
- **`EntitySlot` gains one field:** `actions: SmallVec<[ActionHandlerKey;
  4]>`. The dispose path in `crate::owner_dispose::dispose_owner`
  gains one branch: after entity drop, drain its `actions` and remove
  each from the per-runtime arena. Pattern is identical to the
  observer-removal branch landed in W2.1b.
- **G3 is the seam Phase I + Phase AI both consume.** I6 (keymap)
  produces actions from keys; AI6 (AI-routed palette) produces
  actions from natural language; I5 (plugin host) registers
  handlers from third-party plugins. All three go through the same
  `Runtime::dispatch_action` + `cx.on_action` surface.
- **G4 DrawPhase gains an action-handler boundary to assert on.**
  Action handlers execute during the Update phase (between
  `before_frame` and `request_paint`). Calling `cx.dispatch_action`
  during the Paint phase is a debug-assert panic — same shape as
  DR-045's "writes during Paint are illegal." The G4 DR will cite
  this as one of its write boundaries.
- **G5 `Task<T>` cancellation hooks into action handlers
  naturally.** A handler that spawns a `Task` inside `cx.spawn(...)`
  parents the task to the action's entity; entity dispose cancels
  the task. No additional plumbing — this falls out of DR-045's
  owner-cascade for free.
- **Subscription leak class — same coverage as DR-045.** The
  per-entity `actions` list disposes with the entity. Authors do not
  need to manually deregister handlers when an entity goes away. The
  returned `ActionHandle` is `#[must_use]` and the explicit drop
  unregisters, but the owner cascade is the safety net.
- **Bench surface lands in W3.1b.** Criterion benches in
  `crates/reactive/benches/action.rs`: `action_dispatch_no_handlers`
  (target ≤ 30 ns/op), `action_dispatch_depth_4_focus_path` (target
  ≤ 200 ns/op), `action_register_drop_cycle` (target ≤ 100 ns/op).
  Numbers derived by analogy from entity benches (DR-045
  Consequences); will be re-fit on first real run.

---

## Alternatives considered

### `dyn Action` trait object with manual TypeId — no derive macro

Rejected. The derive macro generates five trait methods; hand-rolling
each action's `boxed_clone` / `as_any` / `eq_action` / `name` /
`namespace` makes a one-field action type a ~25-line file. The
authoring tax compounds across every action in the codebase
(workspaces have dozens). The derive crate is small (~150 LoC for the
macro), lives behind an opt-in feature, and gives plugin authors
exactly the same one-liner ergonomics as sigil-internal authors. The
alternative is correct mechanically but the ergonomics gap is the
reason GPUI ships `actions!`.

### String-keyed actions (`cx.dispatch_action("workspace::SaveAll")`)

Rejected for three reasons: (a) string keys collide silently across
plugins ("save-all" vs "saveAll" vs "save_all" — three identifiers,
one intent, no compiler help); (b) the action payload is untyped, so
`cx.on_action("…", |this, payload: &Foo, cx| …)` either ignores the
payload type or downcasts at runtime; (c) plugin hosts (I5) routinely
want to enumerate "what actions does this plugin provide?" — a
`TypeId` registry answers it; a string-keyed map requires every plugin
to manually register a manifest. Type-keyed is strictly better
across all three axes.

### Single global `enum Action { … }` covering every action in the codebase

Rejected. Two showstoppers: (a) third-party plugins (I5) cannot extend
the enum without patching `sigil-reactive`, which is the worst
possible plugin extension story; (b) the variant count grows
unbounded across the host engine editor + AI surface + IDE primitives + every
consumer, and the enum becomes the central-dispatch anti-pattern
documented in the staff-engineer playbook (variants touching
unrelated subsystems). Per-type actions keyed by `TypeId` decouples
the surface from any single owner.

### Per-runtime global `HashMap<TypeId, Vec<HandlerKey>>` (no per-entity list)

Rejected. Two costs: (a) every dispatch must re-derive "which
handlers are in the focused entity's path" by walking the focus
ancestry and filtering the global vec per-entry — that is
O(handlers_total) per dispatch, not O(focus_depth × handlers_per_entity);
(b) entity disposal must scan the entire global vec to find handlers
to remove (`entity == this`), where the per-entity list disposes in
O(handlers_per_this_entity). The split storage (per-entity list +
per-runtime arena for stable keys) is the boring-correct shape.

### Focus as a reactive `Signal<Option<EntityId>>`

Rejected. Focus is a single value read once per dispatch, not a stream
that observers need to react to. A signal would fire every observer of
focus on every keystroke that moves focus — most observers wouldn't
care. Widgets that *do* care (`on_focus_in`/`on_focus_out`) already
get the notification via the events-crate handler-invocation path; we
keep that wiring unchanged. A plain `Option<FocusKey>` on
`RuntimeState` is the right shape.

### Walk the node tree (`sigil-kernel`) for focus path resolution

Rejected. The node tree is the visual tree; entities are not 1:1 with
nodes (one entity may own zero, one, or several nodes). Walking the
visual tree to find action handlers forces every action-bearing
entity to register against a node, which fails for AI agent-loop
tools (no node), background tasks (no node), and headless tests (no
node tree). The reactive owner tree (DR-005) IS the entity tree and
is the correct spine.

### Priority-attribute override (`#[action(priority = 10)]`)

Rejected. Priority numbers do not compose — when two unrelated
consumers each register a handler for the same action, they each
believe their priority is "the highest one that matters," and the
result is unpredictable. The most-nested-wins resolution is
deterministic and composable: a more-nested entity always wins, and
the answer to "who wins?" is structural (focus path), not numeric
(priority arms-race). This is GPUI's resolution and it is the right
one.

### Registration-order resolution (last-registered wins)

Rejected. Order-sensitivity is the worst kind of subtle bug. The
order in which entities register handlers depends on initialization
order, which depends on owner-tree construction, which depends on the
authoring sequence — all invisible to the consumer. Structural
resolution (focus path) is the only stable answer.

### Scope-stack-explicit overrides (`cx.scope(..., || cx.on_action(...))`)

Rejected. The "explicit scope" model GPUI experimented with early was
discarded for the focus-path model. Explicit scopes require authors
to manually mark every action scope, which couples action-bearing
entity boundaries to non-entity authoring constructs. Focus-path
resolution reuses the entity hierarchy as the scope hierarchy — fewer
concepts, fewer mistakes.

### Implicit always-bubble (action fires every match in the path; explicit stop)

Rejected. The asymmetry is the wrong default: most actions want stop
(per decision 6 reasoning). Always-bubble forces every leaf handler
to call `cx.stop()` to prevent outer firing, which is the same
problem in reverse with more typing. GPUI's "implicit stop, explicit
propagate" matches the 90% authoring case.

### Self-declared key bindings (`#[action(key = "ctrl-s")]`)

Rejected. User-configurable keymaps are non-negotiable for an IDE
(per DR-040 Phase I commitments). An action that hard-codes its key
binding forces the consumer to fork the plugin if they want to remap.
Bindings live outside the action type, in the keymap layer
(deferred to DR-049 / I6).

### Pointer-event absorption (merge `sigil-events` pointer routing into
the action layer)

Rejected. Two responsibilities — input routing vs typed dispatch —
are orthogonal. Merging them couples bugs across layers
(one mis-wired hit-test breaks every plugin's action handlers) and
violates the small-focused-traits rule from the staff-engineer
playbook. Keep them separate; bridge them at one point
(`FocusHandle` extension on the node side, the keymap-engine hook
in `route_key` introduced in DR-049).

---

## Open questions

These are explicitly **not** resolved in this DR. They land in
follow-up DRs or W3.1b implementation discovery.

1. **DR-049 / I6 — Keymap engine + key→action resolution.** The
   `Keymap` type, the key sequence parser, the modal-layer stack, the
   timeout-resolved-as-sequence policy. Consumes G3 via
   `Runtime::dispatch_action`. Scheduled after G3 lands.

2. **Action *payloads* with non-trivial state.** GPUI actions can
   carry data (`struct GoToLine { line: u32, column: u32 }`). The
   `#[derive(Action)]` macro must handle this — payload fields are
   the action's `Send + Sync + 'static` data. The `eq_action`
   generated impl uses `PartialEq` on the fields. Deferred to W3.1b
   implementation; the macro design is straightforward but the
   `eq_action` semantics for payload-bearing actions deserve a quick
   implementer-discovery pass before locking.

3. **Action **observability** for tooling.** Should the runtime emit
   a trace event ("action X dispatched to entity Y, handled by Z")
   for debugger tooling? Probably yes, behind the existing
   `trace::test_hook` mechanism in `sigil-reactive`. Deferred to
   W3.1b — the hook surface is straightforward.

4. **`FocusHandle` cross-entity transfer.** Can entity *A* hand its
   `FocusHandle` to entity *B* (e.g. a wrapper widget delegating focus
   to its inner content)? Provisional answer: yes, via a `FocusHandle`
   clone semantics — the handle is `Copy`-like, and the
   binding stored in `focus_bindings` is the source of truth. Lock
   the semantics in W3.1b after a real consumer (e.g. a `TextField`
   wrapping an editor) hits the case.

5. **Default no-op action for "unknown action" debugging.** When
   `dispatch_action(&Foo)` finds no handler in the path, do we silently
   no-op (GPUI behavior) or fire a `cx.unhandled_action::<Foo>()`
   hook? Provisional: silent no-op with debug-only trace, matching
   GPUI. Re-evaluate if real consumers ask for the hook.

6. **`SendableAction` for cross-thread dispatch from async tasks.**
   `Box<dyn Action>` is `Send + Sync` (the trait requires it).
   Cross-thread dispatch is `cx.spawn(|cx| async move {
   cx.dispatch_action_async(action).await })`. The `dispatch_action_async`
   surface (deferred to G5 / DR-048) shuttles back to the main thread
   via the existing async-bridge pattern.

7. **Action telemetry / analytics.** "How often is action X
   dispatched? By which entity?" This is post-MVP tooling; not in G3
   and not blocking.

---

## References

- **DR-040** — sigil thesis lock (own the foundation; mirror GPUI).
- **DR-045** — G2 entity model (`Entity<T>` + `Context<'_, T>` — the
  substrate this DR builds on).
- **DR-005** — reactive owner tree (the spine used for focus path
  resolution).
- **GPUI parity matrix** —
  `docs/plans/2026-05-16-gpui-parity-integration.md` rows "Action
  dispatch" and "Focus context" in the framework-substrate table.
- **API sketch** — `docs/specs/g3-actions-api-sketch.md` (companion
  to this DR).
- **`crates/reactive/src/lib.rs`** lines 45–79 — module layout the
  action layer extends.
- **`crates/reactive/src/cx.rs`** lines 39–60 — `Context<'_, T>` shape
  the action methods extend.
- **`crates/events/src/router.rs`** lines 217–315 — key dispatch path
  the keymap layer (DR-049) will hook into; the events crate itself
  stays unchanged for G3.
- **`crates/events/src/focus.rs`** lines 10–82 — node-focus shape
  bridged to entity-focus via the `FocusHandle` node extension.
- **GPUI reference** — `zed-industries/zed/crates/gpui` (actions
  module + focus module) and `gpui-ce 0.3.2` for the
  community-extracted shape.

---

## Amendment 2026-05-18 — Action::boxed_clone

W3.2-impl (DR-047 implementation) needed to clone action payloads for
the Paint→Effect deferred-replay queue. Originally landed as an
`A: Clone` bound on `Context::dispatch_action` — every consumer had to
`#[derive(Clone, Action)]` at every dispatch call site, and the bound
leaked into every generic wrapper that wanted to forward the call.

This amendment moves the clone primitive onto the trait itself:

- `Action::boxed_clone(&self) -> Box<dyn Action>` is now a required
  method.
- `Action::as_any(&self) -> &dyn Any` is now a required method —
  trampoline used by the new `dispatch_dyn` path because stable Rust
  has no `dyn Action` → `dyn Any` upcast.
- `#[derive(Action)]` emits both impls automatically; consumers still
  need `#[derive(Clone, Action)]` together because the macro can't
  emit `Clone` itself (would collide with user-written Clone or Copy
  derives). The requirement now sits at type-definition time, not at
  every dispatch call site.
- `Context::dispatch_action<A: Action>` — `Clone` bound dropped.
- `ActionRegistry::dispatch_dyn` added for the dyn-dispatch path the
  deferred-replay queue uses; original monomorphic `dispatch` retained
  for the fast path. `dispatch_entries` was refactored to take
  `&dyn Any` (and a sibling `take_entries_by_id` /
  `dispatch_taken_dyn` was added) so both the monomorphic and dyn
  paths share the same routing core.
- The deferred-write queue (`crates/reactive/src/phase.rs` /
  `cx.rs::dispatch_replay_action`) now captures the action as
  `Box<dyn Action>` via `boxed_clone` and routes the replay through
  `dispatch_dyn`.

No behavioral change for any Ping/Pong/registered action type —
existing test types already had `Clone` from W3.2-impl. The bound was
moved, not added. The benchmark + dispatch tests' hand-written `impl
Action` blocks gained `boxed_clone` and `as_any` implementations
(delegating to `Clone::clone(self)` and `self`).
