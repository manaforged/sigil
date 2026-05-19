---
id: DR-052
status: proposed
date: 2026-05-18
covers: G8 drag-and-drop substrate — intra-app and cross-app drag origination, typed `DragPayload` carrier, `DragSource` / `DropTarget` traits, winit `DroppedFile` / `HoveredFile` ingestion seam, reactive over-target payload signal, drop-target style state slot, `RecordingDnD` test seam
depends_on: [DR-040, DR-045, DR-046, DR-047, DR-049, DR-050, DR-053]
relates: docs/plans/2026-05-16-gpui-parity-integration.md, docs/specs/g8-dnd-api-sketch.md
---

# DR-052 — G8 Drag-and-Drop: Typed Payloads + Reactive Drop Targets

**Status:** Proposed
**Date:** 2026-05-18
**Covers:** The W4.G8 keystone — a drag-and-drop substrate covering
intra-app drag origination (`Cx::drag(payload)` mid event handler),
cross-app drop ingestion (winit `WindowEvent::DroppedFile` /
`HoveredFile` / `HoveredFileCancelled`), the typed `DragPayload` carrier
(intra-app `Any` plus MIME-tagged byte payloads), the `DragSource` and
`DropTarget` traits, the reactive `over` payload signal that lets widgets
style themselves under hover without per-frame callback churn, the
`StateSlot::DragOver` style hook that auto-applies a hover-equivalent
atom overlay on `accepts`-eligible targets, and the `RecordingDnD` test
bridge that captures every drag transition for snapshot assertion.
**Depends on:**
- DR-040 (sigil thesis lock — drag-and-drop is one of GPUI's six pillars;
  mirror its shape: intra-app typed payload + cross-app MIME + custom
  payload types).
- DR-045 (`Entity<T>` / `Context` — `DragSource` and `DropTarget`
  handlers run on an entity's reactive owner; eviction on owner dispose
  is automatic).
- DR-046 (`FocusHandle`, focus path) — drag-over does not change focus;
  drop *can* change focus (drop onto a text field focuses it). The G8
  rule is "drop is the final event that runs on the target subtree;
  focus changes use the existing focus path."
- DR-047 (DrawPhase) — `enter` / `over` / `leave` / `drop` handlers run
  in the Effect phase; the reactive `over` signal is a signal write,
  which lands through the normal Effect drain.
- DR-049 (`StyleAtom` / `StateSlot`) — adds one variant
  `StateSlot::DragOver`, consumed identically to `Hovered` /
  `Focused` / `Pressed` / `Disabled` by the existing overlay resolver.
- DR-050 (a11y) — drop ingestion publishes an AccessKit live-region
  announcement on the target node when the drop completes (e.g.
  "1 file dropped"); routes through the existing live-region drain.
- DR-053 (animator) — drop-target highlight transitions ride the
  `StyleAtom::Transition(TransitionId)` consumer; G8 ships zero animation
  code of its own.
**Unlocks:**
- File-import scenarios (drag a `.png` onto the editor; sigil decodes and
  routes to the active document).
- Wave 5 widget composability — `Reorderable<T>` lists, drag-out tabs
  for split panes, palette-to-canvas tool dispatch, the IDE-class
  layout reflow primitives in sigil-ide-core.
- The matching parity row in `docs/plans/2026-05-16-gpui-parity-integration.md:107`
  — "Drag and drop: intra-app + cross-app + custom MIME ... same shape"
  closes against this DR.

---

## Context

GPUI exposes drag-and-drop through three load-bearing APIs:

1. `cx.start_drag(payload, render_drag_image)` — the source-side hook,
   called from a mouse-move handler when the source has decided the
   pointer has moved far enough to begin a drag. `payload` is `Box<dyn Any>`;
   the render closure produces the floating drag visual.
2. `interactivity.on_drop(|payload: &T, cx| { ... })` — the target-side
   hook, typed by `T: 'static`. The dispatcher walks the over-target
   chain at drop time and invokes the first handler whose type matches
   the payload.
3. Cross-app drops surface through the OS as opaque blobs (file paths
   on every desktop platform, plus optional clipboard-format strings on
   Windows / macOS / Linux), wrapped in the same `DragPayload` type with
   a `Mime` discriminant.

The framework's own job is small but exact: own one in-flight drag at a
time, hit-test the pointer against the target tree each frame the drag
is active, fire enter/leave deltas for the over-target chain, run
`accepts(payload) -> bool` to gate hover feedback, and at drop time
invoke the chain's drop handlers in capture-then-bubble order until one
consumes the event.

Sigil's existing pointer dispatch substrate (`crates/events/src/pointer_dispatch.rs`)
already does the hit-test + capture/target/bubble walk for pointer
events. G8 reuses that walk: the drag-in-flight state is read by an
extension of the existing pointer-move handler, and `enter`/`leave`/`over`
calls fire from the same `dispatch_pointer_path` path. Pure addition,
zero retrofit.

### Reading the existing repo

Files G8 builds on, by path:

- `crates/hal/src/events.rs:21` — `ShellEvent` enum. G8 adds one variant
  `DragDrop(DragDropEvent)` for OS-originated drag events. The variant
  is a separate carrier (not folded into `Pointer`) because it has no
  pointer-button state, no pressure, no kind — it is a different
  vocabulary.
- `crates/events/src/pointer_dispatch.rs:29` — `dispatch_pointer_path`.
  G8 hooks the drag-in-flight check into this dispatch (intra-app
  drags are driven by pointer moves while the source has called
  `cx.drag`).
- `crates/events/src/hit_test.rs:45` — the `hit_test_enabled` /
  region intersection path. G8 reuses unchanged; the drag hit-test is
  the same hit-test.
- `crates/events/src/focus.rs:32` — `FocusManager::set_focus`. G8 does
  not touch focus during drag; an `on_drop` handler may call
  `cx.focus(handle)` if it wants the drop target focused after the drop
  completes.
- `crates/style/src/state.rs:21` — `StateSlot` enum. G8 adds
  `DragOver` as a sixth variant.
- `crates/a11y/src/bridge.rs:23` — `RecordingBridge`. G8 ships a
  `RecordingDnD` test bridge using the exact same pattern.
- `crates/reactive/src/runtime.rs` — `Runtime::begin_effect` drain. G8
  schedules `over` signal writes through the existing deferred-write
  queue per DR-047 §6.

### What does not exist

- **No `crates/dnd/` crate.** No `DragPayload` type. No `DragSource` /
  `DropTarget` traits. No drag-in-flight state on the Runtime. No
  pointer-driven drag origination. No `winit::WindowEvent::DroppedFile`
  ingestion in the shell adapter.
- **No `StateSlot::DragOver`.** Drop-target hover feedback today would
  have to be authored per-widget by subscribing to a hand-rolled signal;
  G8 inverts this to a single overlay slot driven by the framework.
- **No reactive over-target signal.** Widgets that want a custom
  drop-overlay render path (e.g. an outline that traces the dropped
  payload's mime icon) would have to poll the runtime each frame; G8
  surfaces a `Signal<Option<DragPayload>>` so the reactive graph
  re-renders on enter/leave.
- **No cross-app drop wire.** Today the winit shell ignores
  `WindowEvent::DroppedFile`; nothing routes the path into the kernel.

The mistake to avoid is the per-widget drag-state struct. Every
candidate impl that does not own the in-flight drag centrally (Floem
pre-0.3 prototype, an earlier Druid experiment) ended up with two
widgets simultaneously believing they were the drop target on adjacent
pointer-move events, then disagreeing about who fires `leave` on the
next move. The runtime owns the in-flight drag exactly once. Widgets
read; they do not write.

---

## Scope

### In scope (phase 1)

- New crate `crates/dnd/` (package name `sigil-dnd`) owning:
  - `DragPayload` typed carrier (one enum, two variants: `Typed`
    for intra-app `Arc<dyn Any + Send + Sync>` and `Mime` for
    cross-app `(MimeType, Bytes)`).
  - `DragSource` / `DropTarget` traits.
  - `RecordingDnD` test bridge.
  - The runtime-side drag-in-flight state (`DragState`) and its
    per-frame hit-test integration.
- Source-side authoring API: `Cx::drag(payload: DragPayload)` and the
  matching `NodeBuilder::draggable(|cx, ev| { ... })` hook that fires
  when pointer movement crosses the drag-threshold (default 4 device px,
  configurable on the application).
- Target-side authoring API: `NodeBuilder::drop_target(|target_cx|
  DropTarget { ... })`. Trait methods: `accepts`, `enter`, `over`,
  `leave`, `drop`. All four are optional with a default no-op except
  `accepts` (required, default returns `false`).
- Reactive over-target signal: a per-(`NodeId`, payload-type-id) signal
  surfaced through `Cx::drop_payload(node_id) -> Signal<Option<DragPayload>>`.
  Widgets read it during render to gate custom hover visuals.
- Style hook: `StateSlot::DragOver` added to the existing
  `crates/style/src/state.rs` enum. When the in-flight drag's
  over-chain has an `accepts`-eligible target, the runtime sets the
  target's resolved `StateSlot` to `DragOver` for that frame. The
  existing overlay resolver picks up `hover_atoms` / `pressed_atoms` /
  etc. siblings — G8 adds a `drag_over_atoms` field to `Style` parallel
  to those.
- Cross-app drop ingestion: shell adapter consumes
  `WindowEvent::HoveredFile(PathBuf)`, `HoveredFileCancelled`,
  `DroppedFile(PathBuf)`. Each is translated into a
  `ShellEvent::DragDrop(DragDropEvent)` and routed through the runtime
  exactly as an intra-app drag would be — the runtime does not
  distinguish source after translation.
- a11y wiring: on drop, the target node publishes one AccessKit
  live-region announcement via the existing DR-050 §8 drain
  ("1 file dropped" / "{type-name} dropped" — derived from the payload's
  `Display` impl when present, falling back to `"dropped"`).
- Test seam: `RecordingDnD` captures every `start_drag`, `enter`, `over`,
  `leave`, `drop` call into a `Vec<DnDEvent>` for snapshot assertion.

### Out of scope (phase 2 — separate DR amendment or DR-054)

- Custom drag-image rendering (the floating preview that follows the
  pointer). Phase 1 ships a fixed cursor-style highlight; full custom
  preview rendering needs a Layer pool integration that DR-047 hasn't
  unlocked for non-Paint surfaces.
- OS-side outbound drag (sigil → other apps). This requires per-platform
  IDataObject (Windows), `NSDraggingSource` (macOS), XDND
  (Linux/Wayland) origination wiring with the drag-image surface
  composited from sigil's renderer. The winit 0.30 surface area does
  not yet expose outbound drag — tracked at winit issue #720; defer
  until the upstream lands.
- Drag-aware autoscroll (scroll containers that auto-scroll when the
  drag pointer is near their edge). Reusable from intra-app drag state
  but needs the scroll-view widget's threshold contract; defer to the
  W5 scroll-view widget pass.
- Multi-payload drag (drag five files at once). Phase 1 accepts the
  first path; multi-file is one MimeType variant change.
- Drag-cancellation animations. The animator (DR-053) covers fade-out;
  phase 1 just toggles `StateSlot::DragOver` off when the drag exits.
- Touch-originated drag (long-press to begin drag on touch devices).
  Phase 1 is pointer-only; touch arrives with the W5 touch substrate.

---

## Decisions

Seven decisions, frozen for the W4.G8-impl implementer.

### Decision 1: `DragSource` is a closure-typed hook, not a trait widgets implement

```rust
// crates/dnd/src/source.rs
pub struct DragSource {
    /// Called when pointer movement crosses the drag-threshold while
    /// a primary mouse button is held on this node. Return `None` to
    /// veto the drag (the source decided not to drag after all);
    /// return `Some(payload)` to begin.
    pub start: Box<dyn Fn(&mut Cx, &PointerEvent) -> Option<DragPayload> + Send>,
}
```

Authoring:

```rust
NodeBuilder::draggable(|cx, _ev| {
    Some(DragPayload::typed(BookmarkId(42)))
})
```

**Why a closure, not a trait:** the source-side data flow is purely
producer-shaped — "given a pointer event, produce a payload." There is
no shared state between the source closure and any other widget code.
A trait would require a name for the source-struct, an impl block,
and a registration step; the closure folds all three into one builder
call. This mirrors GPUI's `on_drag` shape, which is also closure-based.

The closure runs in the Effect phase (per DR-047 §3) — `Cx::drag` is
the canonical seam, called by the framework with `&mut Cx` so the
closure can read entity state to construct the payload.

**Lifetime story:** the closure is owned by the node's `EventHandlers`
struct (existing). The framework retains it across frames; the
returned `DragPayload` is moved into the runtime's drag-in-flight
state for the lifetime of the drag, then dropped. The closure itself
holds no transient drag context — the only thing it captures is the
entity reference (`Entity<MyState>`) whose state it reads to build the
payload.

**Drag threshold:** the framework debounces "is this a drag yet?" using
a configurable threshold (default 4 device pixels of movement after
`PointerDown` on a `.draggable` node). Below threshold, the source
closure is *not* called — `PointerMove` is delivered as normal. Above
threshold, the closure fires exactly once; the runtime transitions
into "drag in flight" state.

### Decision 2: `DropTarget` is a small trait with optional hooks; `accepts` is the sole required method

```rust
// crates/dnd/src/target.rs
pub trait DropTarget: Send + 'static {
    /// Decide whether the in-flight payload would be accepted by this
    /// target. Called on every over-chain pointer move while the drag
    /// is live. **Must be cheap** — no allocation, no I/O. Return value
    /// gates whether `enter` / `over` / `leave` fire on this target.
    fn accepts(&self, payload: &DragPayload) -> bool;

    /// Pointer entered this target's bounds with an `accepts`-eligible
    /// payload. Defaults to no-op. Fires exactly once per enter.
    fn enter(&mut self, _cx: &mut Cx, _payload: &DragPayload) {}

    /// Pointer moved while inside the target's bounds. Fires per
    /// pointer-move event during hover. Defaults to no-op.
    fn over(&mut self, _cx: &mut Cx, _payload: &DragPayload, _local: Point) {}

    /// Pointer exited this target's bounds (or drag was cancelled while
    /// over the target). Defaults to no-op. Fires exactly once per
    /// leave. **Always fires before any sibling target's `enter`.**
    fn leave(&mut self, _cx: &mut Cx) {}

    /// Payload was dropped on this target. The target consumes the
    /// payload by returning `Drop::Accept(_)`; sibling/ancestor
    /// handlers in the dispatch chain see the residual via `Drop::Pass`.
    fn drop(&mut self, _cx: &mut Cx, _payload: DragPayload) -> DropDisposition {
        DropDisposition::Pass
    }
}

#[derive(Debug)]
pub enum DropDisposition {
    /// Target consumed the drop; dispatch stops, no further targets see
    /// the payload.
    Accept,
    /// Target declined; dispatch continues toward the next handler.
    /// `payload` is returned so the framework can move it forward
    /// without an extra clone for `DragPayload::Mime` (the path bytes
    /// can be large).
    Pass(DragPayload),
}
```

Authoring:

```rust
NodeBuilder::drop_target(|cx_for_init| {
    struct FileSink { entity: Entity<DocumentList> }
    impl DropTarget for FileSink {
        fn accepts(&self, payload: &DragPayload) -> bool {
            payload.mime().is_some_and(|m| m.starts_with("image/"))
        }
        fn drop(&mut self, cx: &mut Cx, payload: DragPayload) -> DropDisposition {
            let Some(path) = payload.into_path() else { return DropDisposition::Pass(payload) };
            self.entity.update(cx, |list, cx| list.import(path, cx));
            DropDisposition::Accept
        }
    }
    Box::new(FileSink { entity: cx_for_init.entity::<DocumentList>() })
})
```

**Why trait, not closures:** unlike `DragSource`, drop targets have
*four* hooks (`accepts` / `enter` / `over` / `leave` / `drop`) sharing
state — the target's "highlighted state" tracked during hover, the
target's accumulated drop-position metadata, the target's reference to
the entity it dispatches into. A trait keeps the four methods in one
spot and gives the implementer a `&mut self` to hold per-target state
without smuggling it through closure captures.

**Why `accepts` returns `bool`, not `Option<Acceptance>`:** the binary
shape matches WCAG drop-zone affordance (over-target either feedbacks
"this would accept" or it does not). Three-valued acceptance (would
accept, conditional, would refuse) is widget-authored visually via
the `over` signal — the framework's job is binary.

**Why `drop` returns `DropDisposition` (consume vs pass):** matches the
capture/target/bubble pattern used by `EventHandlers` (per
`crates/events/src/handlers.rs`). The same dispatch shape the rest of
events uses; G8 does not invent a new propagation model.

**Lifetime story:** the trait impl is constructed once per target
instantiation (when the node is built) and lives as long as the node
lives. The runtime maintains a `Box<dyn DropTarget>` per drop-target
node in the existing `EventHandlers` struct. Owner-tree dispose
(DR-005) collects it automatically on node removal.

### Decision 3: `DragPayload` is one enum with two variants; intra-app uses `Arc<dyn Any>`, cross-app uses `(MimeType, Bytes)`

```rust
// crates/dnd/src/payload.rs
use std::any::{Any, TypeId};
use std::sync::Arc;
use bytes::Bytes;

/// The drag payload — typed for intra-app drags, byte-tagged for
/// cross-app and OS-originated drags.
#[derive(Clone)]
pub enum DragPayload {
    /// Intra-app payload. `value` is an `Arc<dyn Any + Send + Sync>`
    /// so the payload can be inspected via downcast on the target side.
    /// Constructed via [`DragPayload::typed`].
    Typed {
        type_id: TypeId,
        type_name: &'static str,
        value: Arc<dyn Any + Send + Sync>,
    },
    /// Cross-app or OS-originated payload. `mime` is the MIME type;
    /// `bytes` is the raw payload. Path drops are surfaced as
    /// `mime = "application/x-sigil-path"` with `bytes = path.as_os_str()`
    /// re-encoded as UTF-8 lossily.
    Mime { mime: MimeType, bytes: Bytes },
}

impl DragPayload {
    /// Construct a typed intra-app payload.
    pub fn typed<T: Any + Send + Sync>(value: T) -> Self { ... }

    /// Construct a MIME-tagged cross-app payload.
    pub fn mime(mime: impl Into<MimeType>, bytes: impl Into<Bytes>) -> Self { ... }

    /// Convenience: payload representing a file path from a cross-app
    /// drop. Decodes the platform-supplied path.
    pub fn path(path: impl AsRef<Path>) -> Self { ... }

    /// Downcast a typed payload to the concrete type. Returns `None`
    /// for `Mime` variants or type mismatch.
    pub fn downcast<T: Any + Send + Sync>(&self) -> Option<&T> { ... }

    /// MIME of a cross-app payload. Returns `None` for `Typed` variants.
    pub fn mime_type(&self) -> Option<&MimeType> { ... }

    /// Convenience: extract the file path from a path-MIME payload.
    pub fn into_path(self) -> Option<PathBuf> { ... }
}

/// MIME type wrapper. Internally `SmolStr` — most MIME strings are
/// short (`"application/x-sigil-typed"`, `"image/png"`, etc.).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MimeType(SmolStr);
```

**Why one enum, not two types:** the runtime carries exactly one
in-flight payload. A single enum keeps the in-flight slot
monomorphic; the dispatcher matches on the variant exactly once per
drop. Two types would require either a trait object wrapper (slower
double-dispatch) or duplication of every method signature on `DragSource`
/ `DropTarget`. GPUI uses one `Box<dyn Any>` for everything, but
since sigil also has to carry MIME bytes from cross-app drops the
discriminant is unavoidable; one enum is the cheapest representation.

**Why `Arc<dyn Any>`, not `Box<dyn Any>`:** the over-target signal
(Decision 4) hands the payload reference to every accepting target's
reactive subscribers each frame. `Arc` lets subscribers read without
cloning the payload data. The framework retains the master copy until
drop time; subscribers see `Arc::clone` (cheap) instead of
`payload.clone()` (potentially expensive for large `Bytes`).

**Why `bytes::Bytes`, not `Vec<u8>`:** `bytes::Bytes` is already a
workspace dep (used by `sigil-text` for glyph cache + by `sigil-async`
for buffered I/O). Its reference-counted shape gives cheap clones for
the over-target signal fan-out. `Vec<u8>` clones bytewise.

**TypeId carrying:** `type_id` and `type_name` ride alongside `value`
to enable both runtime downcast (the `TypeId` matches the target's
declared expectation) and human-readable diagnostics (`type_name` shows
up in `RecordingDnD` snapshots and in the a11y live-region announcement
fallback).

### Decision 4: Cross-app drop ingestion lands in `sigil-shell`, routes via `ShellEvent::DragDrop`

Three winit events become one sigil event variant:

```rust
// crates/hal/src/events.rs — add to existing ShellEvent enum
pub enum ShellEvent {
    Pointer(PointerEvent),
    Key(KeyEvent),
    Ime(ImeEvent),
    Scroll(ScrollEvent),
    Window(WindowEvent),
    DragDrop(DragDropEvent), // NEW
}

#[derive(Clone, Debug, PartialEq)]
pub enum DragDropEvent {
    /// OS reports a file is being dragged over the window at `position`.
    /// Fires once per file path on initial hover and on each subsequent
    /// move; `path` is the cross-app payload.
    Hovered { path: PathBuf, position: Point },
    /// OS reports the drag exited the window without dropping.
    HoveredCancelled,
    /// OS reports the file was dropped at `position`.
    Dropped { path: PathBuf, position: Point },
}
```

The shell adapter translates:

| winit `WindowEvent`                       | `DragDropEvent`              |
| ----------------------------------------- | ---------------------------- |
| `HoveredFile(path)` + cached cursor pos   | `Hovered { path, position }` |
| `HoveredFileCancelled`                    | `HoveredCancelled`           |
| `DroppedFile(path)` + cached cursor pos   | `Dropped { path, position }` |

**Position resolution:** winit's `HoveredFile` / `DroppedFile` do not
carry a position; the shell adapter caches the last `CursorMoved`
position and pairs it. This is the same trick winit's own examples
use; the cached position is at most one event old on any platform.

**Why the variant on `ShellEvent`, not a side channel:** every event
that affects widget state goes through one queue (`ShellEvent`). A
side channel would invert the existing invariant that pointer-affecting
events arrive in one ordered stream. The drag stream interleaves with
pointer move events naturally — both share the same queue.

**Why ingestion in `sigil-shell`, not a new `sigil-dnd-shell` shim:**
the translation is six lines of winit-to-sigil match; a dedicated shim
would duplicate the cursor-cache plumbing the existing shell already
maintains. `sigil-shell` gains one branch in its event-map; the rest
lives in `sigil-dnd`.

The runtime side: `sigil-dnd` subscribes to `ShellEvent::DragDrop` via
the existing event router. On `Hovered`, the runtime synthesises an
internal "drag in flight" state with `DragPayload::path(path)`, then
runs the same enter/leave/over machinery the intra-app drag would have
triggered. On `Dropped`, the runtime fires `drop` on the target chain
and clears the in-flight state. On `HoveredCancelled`, the runtime
fires `leave` on the current target (if any) and clears.

### Decision 5: Intra-app uses `Cx::drag(payload)` from inside the source closure; cross-app skips origination entirely

The asymmetry the brief calls out is fundamental and not papered over:

**Intra-app origination path:**
1. User presses primary mouse button on a `.draggable` node.
2. Runtime records `(node_id, press_position, modifiers)` in
   `PendingDragOrigin`.
3. User moves pointer; on each move, runtime checks distance from
   press position.
4. When distance exceeds `drag_threshold` (default 4 device px), runtime
   calls the node's `DragSource::start` closure with the current
   pointer event.
5. Closure returns `Some(payload)` → runtime transitions to
   `DragInFlight { payload, source_node_id }`.
6. Subsequent pointer moves dispatch enter/leave/over against the
   target tree using the in-flight payload.

**Cross-app origination path:**
1. OS fires `WindowEvent::HoveredFile(path)` via winit.
2. Shell translates → `ShellEvent::DragDrop(Hovered { path, position })`.
3. Runtime transitions *directly* to `DragInFlight { payload: DragPayload::path(path), source_node_id: None }`.
4. (Steps 1-5 of intra-app are skipped — there is no source closure to
   run; the OS is the source.)

The single `DragInFlight` state covers both. The only field that
differs is `source_node_id: Option<NodeId>` — `None` for cross-app,
which the runtime reads to skip "fire `leave` on source if drag is
cancelled mid-flight inside the source's own bounds" logic.

**Why no synthetic source on cross-app:** any synthetic source node
would have to live somewhere in the tree, would have hit-test
interactions with the drop targets, and would create asymmetry between
"sigil's perception of where the drag began" and "the user's perception
of where it actually came from (outside sigil)." The clean answer is
to leave `source_node_id = None` and document the asymmetry. Cross-app
drags never fire a `source.cancelled` callback; they end with `drop` or
`HoveredCancelled`.

**Cancellation semantics:**
- Intra-app drag, user releases primary button without a valid drop
  target: runtime fires `leave` on the current target (if any), then
  fires the source closure's `cancel` continuation (Phase 1: no
  explicit cancel callback; drag just ends). `DragInFlight` clears.
- Intra-app drag, user presses `Escape`: same path — synthesised
  cancellation. Wires through the existing key-event pipeline (the
  G8 runtime subscribes to `Key(Named(Escape))` while a drag is in
  flight, consumes the event, and cancels).
- Cross-app drag, OS fires `HoveredFileCancelled`: `DragInFlight`
  clears as described in Decision 4.

### Decision 6: Drop-target hover feedback uses `StateSlot::DragOver` (style-driven), plus a `Signal<Option<DragPayload>>` for custom visuals

This decision has two halves; both ship in phase 1.

**Half A — style overlay (the 90% case):**

The existing `StateSlot` enum gains one variant:

```rust
// crates/style/src/state.rs — extension
pub enum StateSlot {
    #[default] Rest,
    Focused,
    Hovered,
    Pressed,
    Disabled,
    DragOver, // NEW
}
```

`Style` gains one field parallel to the existing slot overlays:

```rust
// crates/style/src/struct_api.rs — extension
pub struct Style {
    pub base_atoms: Vec<StyleAtom>,
    pub hover_atoms: Vec<StyleAtom>,
    pub focus_atoms: Vec<StyleAtom>,
    pub pressed_atoms: Vec<StyleAtom>,
    pub disabled_atoms: Vec<StyleAtom>,
    pub drag_over_atoms: Vec<StyleAtom>, // NEW
    // ...
}
```

Authoring via the existing `Styled` builder:

```rust
button("Drop here")
    .bg(Color::neutral_100)
    .drag_over(|s| s.bg(Color::accent_100).border(Color::accent_500))
```

`Styled::drag_over` is the one new method on the trait, parallel to
`Styled::hover` / `Styled::focus` / etc.

When the runtime's drag-in-flight state has a target node, and that
node's `DropTarget::accepts` returned `true` for the current payload,
the runtime sets the node's resolved `StateSlot` to `DragOver` for
that frame. The existing overlay resolver picks up `drag_over_atoms`
and merges them over the base layer — identical mechanism to hover /
focus / pressed / disabled.

**Half B — reactive signal (the 10% case):**

Some widgets need to see the actual payload during hover (e.g. a
tree-view that wants to highlight different drop positions depending
on the dragged item type, or a canvas that wants to render a custom
ghost preview). For those, sigil exposes:

```rust
impl Cx {
    /// Returns a `Signal<Option<DragPayload>>` that reads `Some(payload)`
    /// when a drag is in flight and the pointer is over `node_id`'s
    /// drop-target subtree (and the target's `accepts` returned true).
    /// Returns `None` otherwise. Subscribes the caller for re-render
    /// on enter/leave transitions.
    pub fn drop_payload(&self, node_id: NodeId) -> Signal<Option<DragPayload>>;
}
```

The signal is created lazily on first call per `(node_id, frame)`. The
runtime writes the value during the existing pointer-dispatch path
(no new phase, no new drain). The signal disposes when the node's
owner disposes (DR-005 cascade).

**Why both, not one or the other:**

- Style-only (Half A only) covers the predominant case — "highlight
  this card on drop hover" — without forcing the widget author to read
  a signal, gate on `Some`, and re-render manually. The atom overlay
  is one line of fluent style code.
- Signal-only (Half B only) would force every drop target to author
  its own style transition imperatively. Bad ergonomic regression
  versus the existing hover / focus / pressed pattern.
- Both: the trivial case is one line of `.drag_over(...)`; the complex
  case has a real reactive subscription. Same shape as how `Styled::hover`
  + `Cx::is_hovered(node_id) -> Signal<bool>` co-exist for the hover
  slot (the latter pattern landing under W3.2b — already in repo).

**Animation:** because `StateSlot::DragOver` is a normal slot, the
existing `StyleAtom::Transition(TransitionId)` consumer from DR-053
animates the on/off transition automatically. G8 ships zero animation
code; it gets the animator behaviour by reusing the slot infra.

### Decision 7: Test seam is `RecordingDnD`, parallel to `RecordingBridge` (DR-050 §7) and `RecordingAnimator` (DR-053 §7)

```rust
// crates/dnd/src/testing.rs
#[derive(Debug, Default)]
pub struct RecordingDnD {
    /// Every dnd event observed by the recording bridge in dispatch
    /// order.
    pub events: Vec<DnDEvent>,
}

#[derive(Debug)]
pub enum DnDEvent {
    /// A drag started — source closure returned `Some(payload)`.
    DragStarted { source: NodeId, payload_summary: PayloadSummary },
    /// `DropTarget::enter` fired on `target` with the recorded payload.
    Enter { target: NodeId, payload_summary: PayloadSummary },
    /// `DropTarget::over` fired on `target`.
    Over { target: NodeId, local: Point },
    /// `DropTarget::leave` fired on `target`.
    Leave { target: NodeId },
    /// `DropTarget::drop` fired on `target` and returned `disposition`.
    Drop { target: NodeId, disposition_summary: DispositionSummary },
    /// Drag was cancelled (Escape pressed, button released outside any
    /// accepting target, or OS cancelled the cross-app drag).
    Cancelled,
}

/// One-line payload description for snapshot comparison. Skips the
/// actual payload bytes (which are non-deterministic when they contain
/// e.g. file path absolute prefixes) and records type identity instead.
#[derive(Debug, PartialEq, Eq)]
pub enum PayloadSummary {
    Typed { type_name: &'static str },
    Mime { mime: MimeType, byte_count: usize },
}
```

Tests construct a `Cx`, attach a `RecordingDnD` via
`cx.install_dnd_recorder(&recorder)`, drive a sequence of pointer
events (using the existing `frame_driver` harness shape), and assert
the captured `Vec<DnDEvent>` against expected output.

**Why this shape:** identical to `RecordingBridge` (DR-050 §7) and
`RecordingAnimator` (DR-053 §7). Tests for substrate consistency rely
on the same recording-pattern across G7-G11; a Wave 5 widget author
who has read one knows how to read the others.

**Payload normalisation:** the recorder summarises payloads into
`PayloadSummary` rather than recording them verbatim. This is
load-bearing — file-path payloads on Windows include absolute prefixes
that diverge between dev machines; recording the raw bytes makes
snapshots flake. Recording only `(mime, byte_count)` for `Mime` and
only `type_name` for `Typed` gives stable, semantically-meaningful
goldens.

**Snapshot format:** `Debug` impl on `DnDEvent` produces one-line
output suitable for the existing indented-text snapshot format
(DR-050 §7). The dnd test crate's snapshots live at
`crates/dnd/tests/snapshots/<scenario>.txt`.

---

## Public API surface

The drag-and-drop surface lives in a new `sigil-dnd` crate
(`crates/dnd/` per the path-nested naming convention,
`feedback_crate_naming` memory).

### Crate dependencies

| Dep                  | Why                                                                         |
| -------------------- | --------------------------------------------------------------------------- |
| `sigil-kernel`       | `NodeId`, `Cx`, `Point`, `Entity` (read for payload sources/targets)        |
| `sigil-reactive`     | `Signal`, runtime drag-in-flight state                                       |
| `sigil-hal`          | `ShellEvent::DragDrop`, `PointerEvent`, MIME types                          |
| `sigil-events`       | dispatch hook into pointer-move path (G8 extends pointer dispatch)          |
| `sigil-style`        | `StateSlot`, `Style::drag_over_atoms`, `Styled::drag_over`                  |
| `sigil-a11y`         | live-region announcement on drop                                            |
| `accesskit`          | (via sigil-a11y) `Live::Polite`                                             |
| `bytes`              | `Bytes` carrier for `DragPayload::Mime`                                     |
| `smol_str`           | `SmolStr` backing `MimeType`                                                |

### Public types

```rust
// crates/dnd/src/lib.rs

pub use payload::{DragPayload, MimeType};
pub use source::DragSource;
pub use target::{DropDisposition, DropTarget};

#[cfg(any(test, feature = "test-recording"))]
pub use testing::{DispositionSummary, DnDEvent, PayloadSummary, RecordingDnD};

// crates/dnd/src/payload.rs
pub enum DragPayload {
    Typed { type_id: TypeId, type_name: &'static str, value: Arc<dyn Any + Send + Sync> },
    Mime { mime: MimeType, bytes: Bytes },
}

impl DragPayload {
    pub fn typed<T: Any + Send + Sync>(value: T) -> Self;
    pub fn mime(mime: impl Into<MimeType>, bytes: impl Into<Bytes>) -> Self;
    pub fn path(path: impl AsRef<Path>) -> Self;
    pub fn downcast<T: Any + Send + Sync>(&self) -> Option<&T>;
    pub fn type_name(&self) -> Option<&'static str>;
    pub fn mime_type(&self) -> Option<&MimeType>;
    pub fn into_path(self) -> Option<PathBuf>;
}

pub struct MimeType(SmolStr);
impl MimeType {
    pub const PATH: Self = /* "application/x-sigil-path" */;
    pub const TEXT_PLAIN: Self = /* "text/plain" */;
    pub fn as_str(&self) -> &str;
}

// crates/dnd/src/source.rs
pub struct DragSource {
    pub start: Box<dyn Fn(&mut Cx, &PointerEvent) -> Option<DragPayload> + Send>,
}

// crates/dnd/src/target.rs
pub trait DropTarget: Send + 'static {
    fn accepts(&self, payload: &DragPayload) -> bool;
    fn enter(&mut self, _cx: &mut Cx, _payload: &DragPayload) {}
    fn over(&mut self, _cx: &mut Cx, _payload: &DragPayload, _local: Point) {}
    fn leave(&mut self, _cx: &mut Cx) {}
    fn drop(&mut self, _cx: &mut Cx, _payload: DragPayload) -> DropDisposition {
        DropDisposition::Pass(/* moved-in */)
    }
}

pub enum DropDisposition {
    Accept,
    Pass(DragPayload),
}
```

### `Cx` surface

```rust
impl Cx {
    /// Begin an intra-app drag with `payload`. Called from inside a
    /// `DragSource::start` closure (the only legal site). Calling
    /// outside that context returns `Err(DragError::NotInSource)`.
    pub fn drag(&mut self, payload: DragPayload) -> Result<(), DragError>;

    /// Reactive payload signal for `node_id`'s drop target. Reads
    /// `Some(payload)` during accept-eligible hover; `None` otherwise.
    pub fn drop_payload(&self, node_id: NodeId) -> Signal<Option<DragPayload>>;

    /// Whether the runtime currently has a drag in flight. Reactive.
    pub fn is_dragging(&self) -> Signal<bool>;

    /// Install a `RecordingDnD` for tests. Replaces any prior recorder.
    #[cfg(any(test, feature = "test-recording"))]
    pub fn install_dnd_recorder(&mut self, recorder: Arc<Mutex<RecordingDnD>>);
}
```

### `NodeBuilder` surface

```rust
impl NodeBuilder {
    /// Mark this node as a drag source. `start` is called when the
    /// pointer crosses the drag threshold while a primary button is
    /// held on this node.
    pub fn draggable<F>(self, start: F) -> Self
    where F: Fn(&mut Cx, &PointerEvent) -> Option<DragPayload> + Send + 'static;

    /// Mark this node as a drop target. `init` constructs the target
    /// impl from the build-time `Cx`. The trait impl is retained for
    /// the node's lifetime.
    pub fn drop_target<F, T>(self, init: F) -> Self
    where
        F: FnOnce(&mut Cx) -> T,
        T: DropTarget;

    /// Override the drag-threshold distance for this node (in device
    /// pixels). Default is the application-wide threshold (4px).
    pub fn drag_threshold(self, device_px: u16) -> Self;
}
```

### `Styled` surface

```rust
pub trait Styled: Sized {
    // ... existing methods (hover, focus, pressed, disabled) ...

    /// Apply atoms only when the node is the current drop-over target
    /// for an `accepts`-eligible payload. Composes with the existing
    /// state-slot overlay system.
    fn drag_over(self, f: impl FnOnce(StyleBuilder) -> StyleBuilder) -> Self;
}
```

### `Application` surface

```rust
impl Application {
    /// Application-wide drag-threshold distance (default 4 device pixels).
    pub fn drag_threshold(self, device_px: u16) -> Self;
}
```

### Runtime-private surface

```rust
// crates/dnd/src/runtime.rs (pub(crate))
pub(crate) struct DragState {
    pub(crate) in_flight: Option<DragInFlight>,
    pub(crate) pending_origin: Option<PendingDragOrigin>,
    pub(crate) threshold_device_px: u16,
}

pub(crate) struct DragInFlight {
    pub(crate) payload: DragPayload,
    pub(crate) source_node_id: Option<NodeId>,
    pub(crate) current_target: Option<NodeId>,
    pub(crate) over_chain: SmallVec<[NodeId; 8]>,
}

pub(crate) struct PendingDragOrigin {
    pub(crate) source_node_id: NodeId,
    pub(crate) press_position: Point,
    pub(crate) modifiers: ModifiersState,
}
```

### Error type (crate-local)

```rust
// crates/dnd/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum DragError {
    /// `Cx::drag` called outside a `DragSource::start` closure.
    #[error("Cx::drag must be called from inside a DragSource start closure")]
    NotInSource,
    /// A drag is already in flight; `Cx::drag` was called twice.
    #[error("a drag is already in flight; cancel the existing drag first")]
    AlreadyInFlight,
}
```

One crate-local error type per the rust-hygiene contract. No
re-export of other crates' errors. Bridges to `anyhow` only via the
application binary (never inside `sigil-dnd`).

---

## Implementation phases

### Phase 1 (this DR — W4.G8-impl)

1. New crate `crates/dnd/` with the four modules (`payload.rs`,
   `source.rs`, `target.rs`, `runtime.rs`) plus `testing.rs` behind a
   `test-recording` feature.
2. `ShellEvent::DragDrop(DragDropEvent)` added to `crates/hal/src/events.rs`.
3. Shell adapter (`crates/shell/src/event_map.rs`) translates the three
   winit drag events. Cursor-position cache lives next to the existing
   pointer cache.
4. `StateSlot::DragOver` added to `crates/style/src/state.rs`.
5. `Style::drag_over_atoms` field added to `crates/style/src/struct_api.rs`.
6. `Styled::drag_over` method on the existing trait
   (`crates/style/src/lib.rs`).
7. `NodeBuilder::draggable` / `drop_target` / `drag_threshold` builders
   added to `crates/events/src/builder_ext.rs`.
8. `Cx::drag` / `Cx::drop_payload` / `Cx::is_dragging` methods on
   `crates/reactive/src/cx.rs`.
9. Runtime-side drag dispatch hooks into
   `crates/events/src/pointer_dispatch.rs` (one branch on
   `event.phase()` checking the drag-in-flight state).
10. a11y live-region wiring: drop dispatches an `aria_live` notification
    via the existing DR-050 §8 drain. One new helper in `sigil-a11y`:
    `bridge::announce(cx, node_id, message)`.
11. Tests (lib + integration):
    - `intra_app_drag_threshold_does_not_fire_below_threshold`
    - `intra_app_drag_fires_source_closure_above_threshold`
    - `accept_eligible_target_receives_enter_then_over_then_drop`
    - `non_eligible_target_receives_no_callbacks`
    - `escape_cancels_in_flight_drag`
    - `cross_app_hovered_file_synthesises_drag_in_flight`
    - `cross_app_dropped_file_fires_drop_on_target`
    - `hovered_file_cancelled_fires_leave`
    - `drag_over_state_slot_applies_overlay_atoms`
    - `drop_payload_signal_reads_some_during_eligible_hover`
    - `drop_payload_signal_reads_none_outside_hover`
    - `drop_disposition_pass_continues_to_next_target`
    - `drop_disposition_accept_stops_dispatch`
    - `owner_dispose_clears_drag_in_flight_when_source_disposes`
    - `recording_dnd_captures_full_sequence`
12. Snapshot tests under `crates/dnd/tests/snapshots/`:
    - `basic_drop.txt`
    - `non_accepting_skip.txt`
    - `cancel.txt`
    - `cross_app_path_drop.txt`

### Phase 2 (separate DR — DR-054 candidate)

- Custom drag-image rendering (the floating preview surface). Requires
  a non-Paint Layer hook from DR-047.
- OS-side outbound drag origination (sigil → other apps). Waits on
  winit drag-out surface (upstream issue #720).
- Drag-aware autoscroll (scroll containers auto-scroll near edges).
- Touch-originated drag (long-press to begin).
- Multi-payload drag (drag N items at once).
- Drag-cancellation animations beyond the slot transition (e.g.
  spring-back-to-source ghost).

---

## Tests

Phase 1 ships fifteen unit/integration tests plus four snapshot tests
as enumerated above. Two tests are load-bearing for the substrate
consistency story:

- `drag_over_state_slot_applies_overlay_atoms` — proves the style hook
  (Decision 6 Half A) routes through the existing overlay resolver
  without bespoke wiring.
- `recording_dnd_captures_full_sequence` — proves the test seam
  (Decision 7) captures the exact event-order Wave 5 widgets will
  assert against.

Cross-app drop tests use a synthetic `ShellEvent::DragDrop` injection
(no actual OS drag); the path-translation seam in `sigil-shell` ships
with its own per-platform tests under `crates/shell/tests/` (one each
for Windows, macOS, Linux).

Visual golden: one new golden,
`tests/snapshots/widgets/dropzone_idle_vs_dragover.png`, captured at
the same frame index with and without an in-flight drag over an
accepting target. This is the visual-gate-essential (per
`feedback_visual_gate_essential` memory) check that the
`StateSlot::DragOver` overlay actually renders.

---

## Open questions

These are explicitly not resolved in this DR. They land in follow-up
DRs or W4.G8-impl implementation discovery.

1. **Multi-window drag-out behaviour.** Sigil supports multi-window apps
   (via `sigil-shell` window-handle plumbing). Dragging out of one
   sigil window and into another sigil window is identical to a
   single-window drag at the substrate level (same runtime, same
   payload). But should the source window's `DropTarget::leave` fire
   *before* the target window's `enter`? Provisional: yes, ordered by
   timestamp. Lock when a multi-window test exercises the case.

2. **Drag-source bounds check on drop.** Today `drop` fires on
   whichever target the pointer is over at release. Should the runtime
   also fire a `source.drop_completed(disposition)` callback so the
   source can react (e.g. remove the dragged item from its list on
   `Accept`, leave it on `Pass`)? Provisional: defer to phase 2 —
   widgets that need the callback can subscribe to `Cx::is_dragging`
   and `Cx::drop_payload(any_target)` to infer completion.

3. **Cross-app drop on a non-accepting target.** When the OS drops a
   file onto a window region whose target rejects it, does sigil
   surface a no-op or a system-default "rejection" feedback? Provisional:
   no-op (the file does nothing). System-level rejection animations
   are the OS's job, not sigil's.

4. **Drag-handle vs whole-widget draggability.** A common widget shape
   is "this row is draggable from a specific handle area, not from
   anywhere on the row." Today `.draggable` applies to the whole node;
   handle-only behaviour requires authoring a child node with
   `.draggable` and leaving the parent un-marked. Acceptable for phase
   1; may want a `.draggable_within(child_id)` shortcut in phase 2.

5. **Path payload encoding cross-platform.** Windows `HoveredFile` paths
   are UTF-16; sigil's `PathBuf` is OS-native. Lossy-UTF-8 re-encoding
   in `DragPayload::Mime` may drop characters on non-Latin filenames.
   Provisional: store the raw OS bytes in `Bytes`; the
   `DragPayload::into_path()` helper re-parses via `PathBuf::from`,
   which handles the platform encoding correctly. Tested under
   `crates/dnd/tests/path_encoding_roundtrip.rs`.

6. **Drag-threshold per-source override interaction with application
   default.** What happens if the application sets `drag_threshold(8)`
   and a node sets `drag_threshold(2)`? Provisional: node value wins
   (most-specific overrides). Mirrors how `padding` overlays work in
   the style system.

7. **`Cx::drag` mid-handler ordering.** If a `DragSource::start` closure
   calls `Cx::drag` and *then* mutates entity state, are those mutations
   visible by the time `enter` fires on the first eligible target?
   Provisional: yes — both the `Cx::drag` transition and the entity
   mutations land in the same Effect-phase pass; `enter` fires next
   frame. Lock under `crates/dnd/tests/source_state_mutation_visible.rs`.

8. **Action-routed drop targets.** A drop is naturally tree-routed
   (capture/target/bubble). Should drops also be type-routable through
   the DR-046 action dispatch (`cx.dispatch_action(&DroppedFile(path))`)?
   Provisional: no — drops are positionally bound (the user dropped
   at a specific point), and action dispatch is focus-bound. Keeping
   them separate avoids the "drop fires on the focused element, not
   the pointer target" footgun. Re-evaluate if a consumer actually
   wants this.

---

## References

- DR-040 §4 — GPUI parity thesis lock (drag-and-drop is one of six
  pillars).
- DR-045 — `Entity<T>` / `Context`. The reactive owner that
  `DragSource` / `DropTarget` handlers hang off; eviction cascades
  through the owner tree.
- DR-046 — `FocusHandle` + focus path. G8 does not change focus on
  drop; `on_drop` may call `cx.focus(handle)` if the target wants the
  drop to focus the recipient.
- DR-047 §3 — DrawPhase Effect/Layout/Paint contract. `DragSource::start`
  and `DropTarget::{enter,over,leave,drop}` fire in Effect; signals
  are written through the existing deferred-write queue.
- DR-049 §9 — `StyleAtom::Transition` consumer (locked by DR-053).
  G8 inherits the slot-transition animation infra by adding one
  variant to `StateSlot` — zero new animation code.
- DR-050 §7 — `RecordingBridge` prior-art for the `RecordingDnD`
  test seam. Same pattern, different vocabulary.
- DR-050 §8 — live-region drain at end of Effect. G8 announces drops
  through this drain.
- DR-053 — animator runtime + `StyleAtom::Transition` consumer.
  Drop-target slot transitions ride this without G8 touching the
  animator.
- `crates/hal/src/events.rs:21` — `ShellEvent` enum extended with
  `DragDrop(DragDropEvent)`.
- `crates/events/src/pointer_dispatch.rs:29` — `dispatch_pointer_path`
  reused for drag-in-flight target dispatch.
- `crates/events/src/hit_test.rs:45` — `hit_test_enabled` reused for
  drop-target eligibility hit-test.
- `crates/events/src/focus.rs:32` — `FocusManager::set_focus` reused
  when a drop handler chooses to focus the target.
- `crates/style/src/state.rs:21` — `StateSlot` enum extended with
  `DragOver`.
- `crates/style/src/struct_api.rs` — `Style` extended with
  `drag_over_atoms`.
- `crates/a11y/src/bridge.rs:23` — `RecordingBridge` reference
  implementation for the parallel `RecordingDnD` shape.
- `docs/plans/2026-05-16-gpui-parity-integration.md:107` — parity-matrix
  row this DR closes.
- `feedback_crate_naming` memory — `crates/dnd/` path-nested layout,
  `sigil-dnd` package name.
- `feedback_visual_gate_essential` memory — every UI/render dispatch
  ends with pixel-diff goldens; G8 ships `dropzone_idle_vs_dragover.png`.
- winit 0.30 `WindowEvent::DroppedFile` / `HoveredFile` /
  `HoveredFileCancelled` — https://docs.rs/winit/0.30/winit/event/enum.WindowEvent.html.
- winit issue #720 — outbound drag origination (out of scope for
  phase 1).
- WCAG 2.2 §2.5.7 *Dragging Movements* — pointer alternatives are not
  required for drag-and-drop in phase 1 (the criterion is AA-A; sigil
  targets AA-AA per DR-050 §9 and ships drag as a pointer-only gesture
  with keyboard equivalents authored per-widget — e.g. the palette
  command system in sigil-ide-core covers "move file" without a drag).
