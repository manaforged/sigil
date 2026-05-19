---
id: DR-051
status: proposed
date: 2026-05-18
covers: G7 IME composition substrate — `ImeInputTarget` trait, `Composition` state model, winit → focused-widget ingestion path, candidate-window positioning, AccessKit projection, RecordingImeBackend test seam, Phase-1 platform completeness (Windows TSF + macOS NSTextInputClient + Linux IBus/Fcitx5 via winit)
depends_on: [DR-002, DR-027, DR-045, DR-046, DR-047, DR-050]
relates: docs/plans/2026-05-16-gpui-parity-integration.md, docs/specs/g7-ime-api-sketch.md
---

# DR-051 — G7 IME Composition Substrate

**Status:** Proposed
**Date:** 2026-05-18
**Authors:** sigil-architect (W4.G7 design seat)
**Covers:** The W4.G7 keystone — the public `ImeInputTarget` widget
trait that consents to receive composition events, the `Composition`
reactive state model that carries preedit text + cursor byte range +
commit notifications, the winit `Ime` → `ShellEvent::Ime` →
focused-widget ingestion path (the gap at
`crates/events/src/router.rs:61` where IME events currently fall on
the floor), the `set_ime_cursor_area` rect computation seam (widget
publishes a logical-pixel `caret_rect` per composition update; shell
forwards), the AccessKit live-region projection of preedit text, the
`RecordingImeBackend` test seam mirroring the recording-bridge pattern
from `sigil-a11y` (`crates/a11y/src/bridge.rs:23`), and the Phase-1
platform completeness target — Windows TSF + IMM32 fallback, macOS
NSTextInputClient, Linux IBus + Fcitx5 — all consumed through winit
0.30's existing `Ime` event surface.
**Depends on:** DR-002 (shell + winit owns OS event delivery; ime
ingress lives at the shell boundary), DR-027 (text-input contract;
`TextEdit` is the buffer that absorbs commits; DR-051 supersedes
DR-027's "Phase 14 deferred" line and is the IME substrate DR-027
named), DR-045 (`Entity<T>` + `Context<'_, T>` — composition state
lives as an `Entity<Composition>` per active IME-targeting widget),
DR-046 (`FocusHandle` — only the focused IME target receives
composition events; focus loss commits or cancels per platform), DR-047
(DrawPhase — composition writes are Effect-phase, caret-rect
publication is end-of-Layout, never Paint), DR-050 (AccessKit bridge —
preedit text projects through `Live::Polite` on the same end-of-Effect
drain DR-050 §8 locked).
**Unlocks:** Wave 5 text-bearing widgets (`text_field` v2 IME-aware,
`text_area`, code editor, search input, terminal, ComboBox text input);
the IDE keystone (`sigil-ide-core` already consumes `sigil-events`;
G7's ingestion path is where multilingual code authoring becomes
possible); cross-platform AT-driven text input (NVDA "type with speech"
and VoiceOver dictation hit the same surface). Closes the gap the
parity-integration plan named (`docs/plans/2026-05-16-gpui-parity-integration.md`
row 106).

---

## 1. Context

DR-027 (April 2026) locked the text-input MVP and explicitly deferred
IME composition to a future DR. That DR is this one. Eight months
later the substrate has shifted under it:

- `sigil-hal::ImeEvent` already exists (`crates/hal/src/events.rs:198`)
  with the four variants winit 0.30 emits: `Enabled`, `Preedit(SmolStr,
  Option<(usize, usize)>)`, `Commit(SmolStr)`, `Disabled`. The
  conversion from `winit::event::Ime` happens in
  `crates/shell/src/shell/events.rs:84-93`. The wire is alive.
- `crates/events/src/router.rs:61` drops `ShellEvent::Ime(_)` on the
  floor: `ShellEvent::Window(_) | ShellEvent::Ime(_) => {}`. Nothing
  consumes it. Every IME-aware widget below this line is unreachable.
- DR-046 locked entity focus and `FocusHandle`. The focused node is
  knowable through `FocusManager::focus() -> Option<NodeId>`. The
  routing direction for IME ("send to the focused IME target") is the
  exact dual of the existing key-routing direction
  (`route_key` → focused node's `EventHandlers::invoke_key_any`).
- DR-050 locked the AccessKit bridge and named end-of-Effect as the
  single drain seam. Composition state surfacing to AccessKit must
  ride the same drain — a parallel mechanism would race the existing
  one and produce out-of-order AT announcements.
- DR-047 named the phase order: Effect → Layout → Paint → Idle →
  Effect. Composition state writes happen in Effect; the AccessKit
  publish drains at end-of-Effect; layout reads the updated caret
  position; Paint reads the laid-out caret bounds; only after Paint
  finishes does the framework call `Window::set_ime_cursor_area` for
  the next IME poll. The order is not negotiable.

### What this DR must answer

The gaps the parity plan named (`docs/plans/2026-05-16-gpui-parity-integration.md`
row 106 "IME composition: full TSF/IMM32/NSTextInputClient/IBus/Fcitx5
... DR-042 driven; complete-on-all-three target"):

1. **What does a widget implement to participate in IME composition?**
   Today nothing — `text_field` calls `on_key_any` and inserts
   `KeyEvent::text` into `TextEdit`. There is no trait, no extension,
   no opt-in. A new public surface is needed; DR-050 §3 sets the
   precedent that a widget opts in by setting structured state on its
   node, not by registering a callback.
2. **Where does composition state live?** GPUI keeps composition
   state on the text input model (`InputHandler::marked_text_range`).
   Floem keeps it on the widget (`TextInput::preedit`). The reactive
   substrate's idiom is `Entity<T>` (DR-045); composition is a perfect
   fit for an entity owned by the IME target.
3. **Who computes the caret rect?** winit's `Window::set_ime_cursor_area`
   takes a `Position` + `Size`. The widget knows the *byte offset* of
   the caret; only after Layout + glyph shaping does that resolve to a
   logical-pixel rectangle. The widget must publish the byte offset;
   the framework must do the resolution + winit forwarding.
4. **When does the framework call `set_ime_cursor_area`?** Too early
   (before Layout) → stale rect. Too late (during Paint) → re-entrancy
   per DR-047. End-of-Layout, before Paint, is the only correct seam.
5. **What is the Phase-1 platform completeness target?** The parity
   plan named "complete-on-all-three." winit 0.30 routes Windows
   (TSF via `windows-rs` + IMM32 fallback), macOS (NSTextInputClient),
   and Linux (IBus + Fcitx5 via DBus) through one event vocabulary.
   The bridge is complete-on-all-three *if* every variant of
   `ImeEvent` produces a defined widget-side outcome on every platform.
6. **What does AccessKit hear?** A screen-reader user composing in IME
   needs to hear preedit text as it changes. AccessKit's `Live::Polite`
   on a composition-bound node is the projection. The drain timing is
   DR-050 §8 (end-of-Effect).
7. **How is IME tested?** Same answer DR-050 §10 reached for the a11y
   bridge: a `RecordingImeBackend` (the dual of `RecordingBridge` at
   `crates/a11y/src/bridge.rs:23`) consumes the same routing path,
   captures every composition event, and lets golden tests assert
   against canned `ImeEvent` sequences.

### Reading the existing repo

Files this DR builds on, by path:

- `crates/hal/src/events.rs:196-207` — `ImeEvent` enum, four variants,
  already in the wire vocabulary. G7 does not reshape; G7 adds
  consumers.
- `crates/shell/src/shell/events.rs:84-93` — `WinitShell::push_ime`.
  Already converts every winit `Ime` variant. No drop on the shell
  side. The drop is downstream.
- `crates/events/src/router.rs:61` — the floor where IME currently
  dies. G7 replaces the `_ => {}` arm with `ShellEvent::Ime(event)
  => self.route_ime(cx, event)`.
- `crates/events/src/focus.rs` — `FocusManager::focus()` returns the
  currently focused `NodeId`. G7 reads it to find the IME target;
  composition events go to that node and nowhere else.
- `crates/events/src/handlers.rs:15-32` — `EventHandlers` extension.
  G7 adds a parallel `ImeHandlers` extension (not a field on
  `EventHandlers` — composition is a different concern from pointer +
  key + scroll + focus and a 14-field god struct is the rejected
  shape per the staff hygiene contract).
- `crates/sigil/src/widgets/text_edit.rs` — the `TextEdit` editor.
  `insert_str` already takes `&str`; `Commit` text feeds straight in.
  Preedit text is a *separate visual layer* that does not touch the
  buffer until commit.
- `crates/kernel/src/builder.rs` — `NodeBuilder`. G7 adds three
  builder methods: `ime_target(Entity<Composition>)`,
  `on_ime_commit(impl Fn(&str) + 'static)`,
  `on_ime_preedit(impl Fn(&Composition) + 'static)`.
  Authoring shape mirrors DR-046 / DR-049 conventions (one-word verbs,
  no `with_`).
- `crates/a11y/src/bridge.rs:23` — `RecordingBridge`. The prior-art
  pattern for `RecordingImeBackend`. Same shape: `Vec<event>` capture,
  FIFO injection, infallible publish.
- `crates/reactive/src/entity.rs` — `Entity<T>` and `Context<'_, T>`.
  Composition state is `Entity<Composition>`; the IME target reads
  and writes through `Context::update_entity`.

### What does not exist

- **No `ImeInputTarget` trait.** Widgets cannot declare "I receive
  composition events."
- **No `Composition` type.** No struct holds preedit text + cursor
  byte range + commit history per active IME session.
- **No `ImeHandlers` node extension.** `EventHandlers` covers pointer
  / key / scroll / focus — not IME.
- **No router method `route_ime`.** `route_pointer` / `route_key` /
  `route_scroll` are the parallels; no IME equivalent.
- **No caret-rect publication path.** `set_ime_cursor_area` is never
  called; on Windows + macOS this means the candidate window pops up
  at (0, 0) screen-relative or at the cursor's last text-insertion
  point — both visibly wrong.
- **No `set_ime_allowed` integration.** Today the shell never tells
  winit "this surface accepts IME"; the OS defaults are correct on
  Linux but on Windows the IME stays off unless the application opts
  in.
- **No `RecordingImeBackend`.** Tests can synthesise `ShellEvent::Ime`
  values but no captured surface receives them.

### The mistake to avoid

The retrofit failure is platform-specific code paths inside widget
implementations. `text_field` should not contain a `#[cfg(target_os =
"windows")]` block. The bridge owns the indirection: every widget
authors `Composition` writes, the framework routes events through
the focused-target lookup, and one platform-specific module
(`sigil-shell`'s winit adapter) speaks to the OS. This is the same
shape DR-050 §1 mandated for AccessKit: one consumer crate, one
projection surface, widgets author only the structured state.

---

## 2. Scope

### In scope (Phase 1, this DR + W4.G7-impl ticket)

- Public `ImeInputTarget` trait, `Composition` reactive state, three
  new `NodeBuilder` methods (`ime_target`, `on_ime_commit`,
  `on_ime_preedit`), the `ImeHandlers` node extension.
- Router method `route_ime` that finds the focused IME target and
  dispatches composition events; `EventRouter::ime_caret_rect` query
  the shell calls end-of-Layout.
- Shell `set_ime_allowed(window_id, bool)` driven by focus change
  (focus enters an IME target → enable; leaves → disable).
- Shell `set_ime_cursor_area(window_id, rect)` driven by end-of-Layout
  drain after composition state writes.
- AccessKit `Live::Polite` projection of preedit text on the
  composing node, draining through DR-050 §8's end-of-Effect path.
- `RecordingImeBackend` test seam (dual of `RecordingBridge`).
- `text_field` v2 widget update — opt into IME by attaching a default
  `Entity<Composition>` when constructed; commit text into `TextEdit`;
  render preedit as an overlay (rendering is W4.G7-impl, not this DR's
  concern).
- Phase-1 platform completeness target: every `ImeEvent` variant has
  a defined widget outcome on Windows (TSF + IMM32), macOS
  (NSTextInputClient), Linux (IBus + Fcitx5) — *consumed through
  winit 0.30's translation layer*, not through raw platform APIs.

### Out of scope (deferred to Phase 2 / future DR)

- **Direct TSF / IMM32 / NSTextInputClient / IBus / Fcitx5 access.**
  winit 0.30 owns the platform translation. Sigil consumes the
  `Ime` event vocabulary winit ships and does not link any
  platform-specific text-services library directly. The "complete-on-
  all-three" target is met by ensuring every winit-emitted variant
  has a well-defined widget outcome, not by re-implementing the
  platform clients. If winit's translation has a per-platform bug
  (e.g. early winit 0.30 dropped Fcitx5 commit events on some
  distros) the workaround lives in `sigil-shell`, not in widget code.
- **Candidate window styling.** Native IME UI is OS-owned; sigil does
  not render its own candidate list in Phase 1. Custom candidate UI
  (the GPUI ChatGPT-style suggestion popover pattern) is a Phase 2
  concern that builds on the same `Composition` state.
- **Conversion / candidate selection through application-level
  shortcuts.** Arrow-key candidate navigation happens inside the OS
  IME; sigil does not intercept. If a future widget (terminal,
  game-chat) needs to suppress IME during certain modes, it sets
  `ime_target` to `None` and falls back to raw `KeyEvent::text`.
- **Voice dictation.** macOS Voice Control and Windows Voice Access
  dispatch through AccessKit's `Action::SetValue` (DR-050 §8). They
  do not flow through `ImeEvent`. G7 does not need a separate seam.
- **Multi-line composition coordination across line breaks.** Phase 1
  treats every `Commit` as a single-paragraph insertion at caret;
  composition that spans hard line breaks (rare in CJK; never in
  Latin-script IME) is handled by the OS pre-flattening to one
  string. If a real consumer hits the limit, escalate.
- **Pre-edit autocomplete suggestions.** The sigil-side "IDE
  autocomplete during IME composition" interaction (IntelliJ-style)
  is `sigil-ide-core`'s concern, not the substrate's.

---

## 3. Decisions

Seven decisions, frozen for the W4.G7-impl implementer.

### Decision 1 — Public IME surface: `ImeInputTarget` trait on widget builders + three `NodeBuilder` methods

A widget participates in IME composition by:

1. **Holding a `Composition` entity in its builder.** Idiomatically
   constructed via `Composition::new()` and stored on the builder
   struct (e.g. `TextFieldBuilder::composition: Entity<Composition>`,
   created lazily on first `.ime_target()` call or `.ime_enabled(true)`
   call).
2. **Calling `.ime_target(composition_entity)` on its root
   `NodeBuilder`.** This attaches the `ImeTargetExt { composition:
   Entity<Composition> }` kernel extension to the node. The router
   reads this extension when dispatching IME events.
3. **Optionally registering `on_ime_commit` and `on_ime_preedit`
   handlers** for behaviour beyond the default
   `Composition::apply_commit` / `Composition::apply_preedit` writes.
   The default behaviour is enough for `text_field`; richer consumers
   (a code editor that wants to validate IME input against syntax)
   register handlers.

The `ImeInputTarget` trait is a marker on **widget builders**, not on
nodes:

```rust
pub trait ImeInputTarget {
    /// Returns the composition state this widget reads + writes.
    fn composition(&self) -> &Entity<Composition>;

    /// Whether composition is currently allowed on this widget.
    /// Defaults to true. Widgets that disable IME programmatically
    /// (terminal in raw mode, password input) override this.
    fn ime_enabled(&self) -> bool { true }
}
```

This is the dual of `Styled` from DR-049 — a builder-level trait, not
a kernel-level trait, mirroring the GPUI-shaped fluent surface.
Widgets that implement `ImeInputTarget` automatically participate;
widgets that don't never receive `ImeEvent` and never trigger
`set_ime_allowed(true)` on focus.

Method naming follows the project's runtime-API convention (one-word
verbs, no `with_`, configure-as-noun): `ime_target`, `ime_enabled`,
`on_ime_commit`, `on_ime_preedit`. Not `with_ime_target` or
`set_ime_target`.

**Rejected: callback-only registration (`on_ime_event(handler)`).**
Composition is *state*, not an event stream. A widget that only
registers a callback has to re-derive preedit + cursor + active-state
on every event; routing that through `Entity<Composition>` lets every
consumer read the same shape, and the AccessKit projection can read
it without subscribing to handlers.

### Decision 2 — Composition state model: `Composition` is one struct, stored in an `Entity<Composition>`

```rust
pub struct Composition {
    /// Whether the OS-side IME is currently active for this target.
    /// Mirrors winit's Enabled/Disabled lifecycle.
    pub active: bool,

    /// In-progress preedit text. Empty string when no composition is
    /// active or between Preedit and Commit.
    pub preedit: SmolStr,

    /// Byte-indexed cursor range inside `preedit`, if the platform
    /// supplies one. Phase-1 platforms all do.
    pub preedit_cursor: Option<(usize, usize)>,

    /// Monotonic counter incremented every time `apply_commit` is
    /// called. Lets observers detect "a commit happened this frame"
    /// without holding a stale event reference.
    pub commit_seq: u64,

    /// Last committed text (for AccessKit live-region projection).
    /// Cleared at start of next composition.
    pub last_commit: SmolStr,
}
```

The struct is **one** struct, not split. The fields are tightly
coupled — preedit and preedit_cursor go together every frame; commit
and commit_seq are read together. Splitting (`PreeditState` vs
`CommitState`) would require widgets to update two entities in
lockstep; observers would have to subscribe to both. The "is one
struct" call is the right call.

Composition is wrapped in `Entity<Composition>` (DR-045's reactive
container). This gives:

- Free `Composition: Send + 'static` storage on the runtime arena.
- Observer fan-out for free (the AccessKit projection subscribes
  exactly the same way as `Signal<String>` projections from DR-050
  §4).
- Reactive composition with downstream signals (text-field
  `display_text: Derived<String>` that splices preedit into the buffer
  for rendering).
- Owner-cascade disposal (DR-005) — when the widget node disposes,
  its `Entity<Composition>` disposes too, so no observer leaks.

**Per-focus-context owned, not global.** Each IME-targeting widget
owns its own `Entity<Composition>`. There is no global
`current_composition: Option<Entity<Composition>>` on the runtime.
The router looks up "which composition do I write to?" by
(a) reading `FocusManager::focus()` for the current node,
(b) reading the `ImeTargetExt` extension on that node,
(c) writing through `Context::update_entity(comp.key, |c| ...)`.
If focus moves between IME-targeting widgets mid-composition, the
old widget's composition is finalized by the OS (winit emits
`Disabled` + a `Commit` of whatever was pending, or just `Disabled`
on cancellation depending on platform); the new widget starts fresh.

### Decision 3 — Platform-event ingestion path: shell → router → focused IME target, no new crate

`ShellEvent::Ime` already exists in `sigil-hal`. `WinitShell::push_ime`
already converts winit events. The gap is in `sigil-events::EventRouter`,
not anywhere else. **No new crate.** `sigil-ime` was considered and
rejected (see §8 Alternatives).

The dispatch path:

1. winit emits `WindowEvent::Ime(ime)` → `WinitShell::push_ime` →
   `ShellEvent::Ime(ImeEvent)` pushed onto the shell event queue.
2. Application drains via `Shell::drain_events()`; passes each event to
   `EventRouter::route(&mut cx, &layout, event)`.
3. `EventRouter::route` matches `ShellEvent::Ime(event) =>
   self.route_ime(cx, event)`.
4. `route_ime` reads `self.focus.focus()` (the focused `NodeId`); if
   `None`, drops the event (no IME target).
5. `cx.get(node_id).and_then(|n| n.get_extension::<ImeTargetExt>())`
   yields the `Entity<Composition>` if the node opted in via
   `.ime_target(comp)`. If absent, drop the event with a
   `tracing::trace!` (the OS sent us composition state but no widget
   is configured to consume it; on Phase-1 platforms this should be
   impossible because `set_ime_allowed` only got toggled on for
   IME-target focus, but defensive).
6. For each variant:
   - `Enabled` → `composition.update(|c| c.active = true)`.
   - `Preedit(text, range)` → `composition.update(|c| { c.preedit =
     text; c.preedit_cursor = range; })`. Plus invoke any registered
     `on_ime_preedit` handlers on the `ImeHandlers` extension.
   - `Commit(text)` → `composition.update(|c| { c.preedit = SmolStr::
     default(); c.preedit_cursor = None; c.last_commit = text.clone();
     c.commit_seq += 1; })`. Plus invoke `on_ime_commit` handlers
     (which the default `text_field` implementation uses to call
     `TextEdit::insert_str(text)`).
   - `Disabled` → `composition.update(|c| { c.active = false;
     c.preedit = SmolStr::default(); c.preedit_cursor = None; })`.

Routing IME events through `EventRouter` (not a separate
`ImeRouter`) keeps the focus-truth in one place. Adding a parallel
router would require focus synchronisation across two structures —
the exact dual-truth problem DR-050 §5 banned for entity-vs-node
focus.

### Decision 4 — Candidate-window positioning: widget publishes a `CaretAnchor`; framework resolves at end-of-Layout

Two-step:

**Step A (widget side, Effect phase).** The IME-target widget
publishes a logical-coordinate caret anchor as a node extension:

```rust
pub struct CaretAnchorExt {
    /// Logical-coordinate rect around the caret. Origin is
    /// node-relative; framework projects to window-relative.
    pub rect: LogicalRect,
}
```

The widget computes this from its `TextEdit::caret` byte offset and
its laid-out glyph runs. `text_field` writes the extension every
frame composition is active (it's cheap; a single `LogicalRect`).
Widgets that don't write `CaretAnchorExt` get no candidate-window
positioning — the OS falls back to the window-origin default, which
is visibly wrong but doesn't crash.

**Step B (framework, end-of-Layout phase).** A new method
`EventRouter::ime_caret_rect(cx, layout) -> Option<(WindowId,
PhysicalRect)>` reads the focused node, finds `CaretAnchorExt`,
combines it with the node's absolute layout offset (already computed
by Layout), converts to physical pixels via the window's scale
factor, and returns the rect.

`Shell::set_ime_cursor_area(window_id, rect)` is called once per
frame, end-of-Layout, after the rect resolves. If the rect is the
same as last frame, the shell adapter skips the winit call (winit
handles this idempotently anyway, but skipping saves one DBus
round-trip per frame on Linux + IBus).

**Why end-of-Layout, not Effect:** Effect runs before Layout; the
node's absolute origin isn't known until Layout finishes. Calling
`set_ime_cursor_area` during Effect would use last frame's bounds,
producing a one-frame lag in candidate-window placement during fast
typing.

**Why end-of-Layout, not end-of-Paint:** Paint is read-only per DR-047
§6. Calling a shell-side method during Paint introduces the
re-entrancy DR-047 banned. End-of-Layout is the first moment the rect
is correct *and* the framework is allowed to call out.

### Decision 5 — Phase-1 platform completeness: winit 0.30's `Ime` vocabulary on all three platforms, no direct platform-API access

The parity-plan target ("TSF/IMM32/NSTextInputClient/IBus/Fcitx5
... complete-on-all-three") is met by:

1. winit 0.30 ships these adapters internally: `winit::platform::windows`
   handles TSF + IMM32 fallback; `winit::platform::macos` handles
   NSTextInputClient; `winit::platform::linux` (x11 + wayland)
   handles IBus + Fcitx5.
2. Every winit `Ime` variant is translated by `WinitShell::push_ime`
   into one of the four `ImeEvent` variants. No information is lost
   that any phase-1 widget consumes.
3. Phase-1 widget outcome contract — every `ImeEvent` variant
   produces a defined state mutation on `Composition` and a defined
   downstream effect:

| Variant     | Composition mutation                                 | Downstream effect                                            |
| ----------- | ---------------------------------------------------- | ------------------------------------------------------------ |
| `Enabled`   | `active = true`                                      | AccessKit no-op; render: caret shows composition cursor      |
| `Preedit`   | `preedit = text; preedit_cursor = range`             | AccessKit Live::Polite announces preedit; render overlays    |
| `Commit`    | `preedit = ""; last_commit = text; commit_seq += 1`  | `on_ime_commit` runs (default: `TextEdit::insert_str`)       |
| `Disabled`  | `active = false; preedit = ""`                       | AccessKit Live::Polite announces final state; render clears  |

The contract is *exhaustive* — every variant has a defined outcome on
every platform. winit's translation absorbs platform-specific quirks
(e.g. macOS NSTextInputClient sometimes sends a Preedit+Commit pair
where IBus would send only Commit; winit normalises). Any
platform-specific bug discovered post-Phase-1 lives in `sigil-shell`'s
adapter (e.g. a workaround in `push_ime` to coalesce duplicate
Disabled events).

**The "complete-on-all-three" gate is a test gate, not an API gate:**
the `RecordingImeBackend` test harness (§Decision 7) replays canned
event sequences captured from each platform during Phase-1
implementation. A test passes when the same sequence produces the
same widget-side state on every host running CI. The gate is
verified by the W4.G7-impl CI matrix (Windows + macOS + Linux
runners), not by sigil duplicating winit's platform code.

### Decision 6 — AccessKit projection: preedit text → `Live::Polite` on the IME-target node, drained end-of-Effect

DR-050 §8 locked the AccessKit live-region drain at end-of-Effect.
G7 reuses the same drain, no new mechanism:

- When the widget calls `.ime_target(composition)`, the framework
  attaches an observer on `Entity<Composition>` (analogous to the
  signal observers DR-050 §4 already installs). Observer effect: mark
  the node dirty in `RuntimeState::a11y_dirty` *and* push onto
  `RuntimeState::live_announcements` whenever `composition.preedit`
  changes or `composition.commit_seq` increments.
- The end-of-Effect drain reads `RuntimeState::live_announcements`
  exactly as DR-050 §8 specified. For composition nodes, the
  `accesskit::Node::set_value` is set to a synthesized string:
  `format!("{}{}", buffer_before_caret, composition.preedit)` — the
  preedit text appears inline with the buffer text so the AT reads
  "email at example dot... <preedit>" as one stream.
- AccessKit's `Live::Polite` is attached to the IME-target node
  automatically when `.ime_target(comp)` is called (the framework
  sets `A11yProps::live = Some(Live::Polite)` if not already set).
  Widgets can override via `.aria_live(Live::Assertive)` if the
  semantic is more urgent (rare; chat input on a focused message =
  Polite; password field with masked composition = Polite).

The projection rule: **only preedit text broadcasts to AccessKit
mid-composition; commits flow through the buffer's existing
`text_value_signal` and don't need an extra announcement.** The
existing `text_field` already has `A11yProps::text_value =
Some(Signal<String>)` (DR-027), which AccessKit re-reads on every
buffer mutation (DR-050 §4 observer). G7's projection is purely the
preedit channel; commit announcement is free via the buffer path.

This also resolves a subtle WCAG 4.1.3 concern: announcing every
preedit keystroke without the polite throttling would spam the AT;
`Live::Polite` lets the AT decide when to interrupt the current
utterance (typically at sentence boundaries or after a configurable
delay).

### Decision 7 — Test strategy: `RecordingImeBackend` + golden tests against canned `ImeEvent` sequences

Mirroring `crates/a11y/src/bridge.rs:23`:

```rust
#[derive(Debug, Default)]
pub struct RecordingImeBackend {
    /// Every IME event the router dispatched, in order.
    pub dispatched: Vec<(NodeId, ImeEvent)>,
    /// Every caret-rect publication, in order.
    pub published_rects: Vec<(WindowId, PhysicalRect)>,
    /// Every set_ime_allowed call, in order.
    pub allowed_toggles: Vec<(WindowId, bool)>,
}
```

`RecordingImeBackend` is **not** an implementation of a trait the
router takes generically (the router does not abstract over backends
— it just calls one method on its `&mut Cx`). The recording happens
at the `EventRouter` boundary by:

1. A test harness shell (already exists via `WinitShell`'s headless
   test variant pattern) implementing `Shell::set_ime_cursor_area`
   and `Shell::set_ime_allowed` by pushing into the backend's
   `published_rects` + `allowed_toggles`.
2. Tests calling `EventRouter::route(cx, layout, ShellEvent::Ime(...))`
   directly with synthetic events. The router's
   `route_ime` records into the backend via a hook
   (`EventRouter::with_ime_recorder(backend)` — opt-in, debug-only,
   compiled out of release).
3. Golden tests in `crates/sigil/tests/ime_*.rs` replay canned
   sequences (e.g. `tests/fixtures/ime/japanese_kana.json` —
   `["Enabled", "Preedit(\"k\", (1,1))", "Preedit(\"か\", (1,1))",
   "Commit(\"か\")", "Disabled"]`) and assert the final widget
   state.

Sequence capture happens once per platform during W4.G7-impl
landing — the implementer types canned phrases through real OS IME
into a capture binary that dumps `ImeEvent` sequences as JSON; those
fixtures land in `tests/fixtures/ime/*.json` and become the
phase-1 platform-completeness regression set.

The pattern (recording backend + JSON fixtures + golden tests) is
the same pattern DR-050 §10 used for AccessKit (`assert_a11y_tree!`
+ snapshot files). Mirroring the structure means the test author
cognitive overhead is zero — anyone who's authored an a11y snapshot
can author an IME golden.

---

## 4. Public API Surface

Full signatures. The implementer translates these to running code
under W4.G7-impl.

### 4.1 `sigil_kernel::Composition` + `Entity<Composition>` (in `sigil-reactive` re-export)

```rust
/// State of an in-progress input-method composition for one
/// IME-targeting widget.
///
/// One [`Composition`] lives in an [`Entity<Composition>`] owned by
/// the widget. Routing reads + writes through the entity; observers
/// (rendering, accessibility) subscribe via the entity's reactive
/// fan-out.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Composition {
    pub active: bool,
    pub preedit: SmolStr,
    pub preedit_cursor: Option<(usize, usize)>,
    pub commit_seq: u64,
    pub last_commit: SmolStr,
}

impl Composition {
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// True when the OS has signalled IME composition is active
    /// (winit `Ime::Enabled` received, no `Ime::Disabled` since).
    #[must_use]
    pub const fn active(&self) -> bool { self.active }

    /// Current preedit text. Empty when no composition is in flight.
    #[must_use]
    pub fn preedit(&self) -> &str { self.preedit.as_str() }

    /// True when a commit was applied during the current frame.
    /// Resets at end-of-Effect drain.
    #[must_use]
    pub fn just_committed(&self, last_seen_seq: u64) -> bool {
        self.commit_seq != last_seen_seq
    }
}
```

`Composition` lives in `sigil-kernel::ime` (new module). The
re-export `sigil_reactive::Composition` is a convenience because the
public consumer surface (widget builders) imports from
`sigil_reactive` for entity types.

### 4.2 `sigil_kernel::ImeTargetExt` (kernel extension)

```rust
/// Node extension attached when a widget calls
/// `NodeBuilder::ime_target(...)`. The router reads this to
/// dispatch composition events to the right entity.
#[derive(Debug)]
pub struct ImeTargetExt {
    pub composition: Entity<Composition>,
}
```

Visibility: `pub` because `sigil-events::EventRouter` (a different
crate) reads it via `Node::get_extension::<ImeTargetExt>`.

### 4.3 `sigil_kernel::CaretAnchorExt` (kernel extension)

```rust
/// Logical-coordinate caret rect the widget publishes per-frame
/// during composition. Framework reads at end-of-Layout to drive
/// `Shell::set_ime_cursor_area`.
#[derive(Clone, Copy, Debug)]
pub struct CaretAnchorExt {
    pub rect: LogicalRect,
}
```

`LogicalRect` is the existing `sigil_hal::LogicalRect` (already in
hal). Visibility `pub` for the same router-read reason.

### 4.4 `sigil_kernel::NodeBuilder` (three new methods)

```rust
impl NodeBuilder {
    /// Declare this node as the IME composition target. The
    /// `Composition` entity receives state writes for `Enabled` /
    /// `Preedit` / `Commit` / `Disabled` events while this node is
    /// focused.
    ///
    /// Attaches [`ImeTargetExt`] to the node and (if not already
    /// set) `A11yProps::live = Some(Live::Polite)` so preedit text
    /// projects to assistive technology.
    #[must_use]
    pub fn ime_target(self, composition: Entity<Composition>) -> Self;

    /// Run `h` when a `Commit` event lands on this node. Runs *after*
    /// the default `Composition::apply_commit` write (the entity is
    /// already updated when the handler fires).
    #[must_use]
    pub fn on_ime_commit<H>(self, h: H) -> Self
    where
        H: Fn(&str) + 'static;

    /// Run `h` whenever the active `Composition` mutates (preedit
    /// text change, cursor move, commit, enable/disable). Receives
    /// a borrow of the entity's current state.
    #[must_use]
    pub fn on_ime_preedit<H>(self, h: H) -> Self
    where
        H: Fn(&Composition) + 'static;
}
```

Naming notes per `feedback_runtime_api_naming`: one-word verbs
(`ime_target`, not `set_ime_target`); no `with_` prefix; configure-as-
noun for the entity (`ime_target(composition)`), event handlers as
`on_<event>` matching `on_pointer_*` / `on_key_*` convention.

### 4.5 `sigil_events::ImeHandlers` (node extension)

```rust
type ImeCommitHandler = Box<dyn Fn(&str) + 'static>;
type ImePreeditHandler = Box<dyn Fn(&Composition) + 'static>;

/// IME handlers attached to a node through the extension bag.
///
/// Separate from [`EventHandlers`] because composition is an
/// orthogonal concern from pointer/key/scroll/focus and merging
/// into one struct violates the staff-engineer god-struct rule.
#[derive(Default)]
pub struct ImeHandlers {
    commit: Vec<ImeCommitHandler>,
    preedit: Vec<ImePreeditHandler>,
}

impl ImeHandlers {
    pub(crate) fn add_commit(&mut self, h: ImeCommitHandler);
    pub(crate) fn add_preedit(&mut self, h: ImePreeditHandler);
    pub(crate) fn invoke_commit(&self, text: &str);
    pub(crate) fn invoke_preedit(&self, comp: &Composition);
}
```

Visibility: struct is `pub`; all methods are `pub(crate)` — only
`sigil-events::EventRouter::route_ime` invokes them. Builder
methods on `NodeBuilder` (in §4.4) install via the
`NodeImeExt` trait below.

### 4.6 `sigil_events::NodeImeExt` (builder extension trait — implementation side)

```rust
pub trait NodeImeExt {
    #[must_use]
    fn on_ime_commit<H>(self, h: H) -> Self where H: Fn(&str) + 'static;

    #[must_use]
    fn on_ime_preedit<H>(self, h: H) -> Self where H: Fn(&Composition) + 'static;
}

impl NodeImeExt for NodeBuilder { /* ... */ }
```

Mirrors `NodeEventsExt` from `crates/events/src/builder_ext.rs`. The
`NodeBuilder::ime_target` method itself lives on the kernel
builder (it attaches `ImeTargetExt`, no handler installation, so it
doesn't need to cross the events crate).

### 4.7 `sigil_events::EventRouter::route_ime` + caret-rect query

```rust
impl EventRouter {
    /// Dispatch one IME event to the focused IME target. No-op if
    /// no node is focused or the focused node has no
    /// [`ImeTargetExt`].
    pub fn route_ime(&mut self, cx: &mut Cx, event: ImeEvent);

    /// Return the focused IME target's caret rect, projected to
    /// physical-pixel window coordinates. Called at end-of-Layout
    /// before the shell forwards to `Shell::set_ime_cursor_area`.
    /// `None` when no IME target is focused or the target has no
    /// `CaretAnchorExt` for this frame.
    #[must_use]
    pub fn ime_caret_rect(
        &self,
        cx: &Cx,
        layout: &LayoutEngine,
        scale: Scale,
    ) -> Option<PhysicalRect>;
}
```

The router's existing `route` method gains one arm:

```rust
match event {
    // ... existing arms ...
    ShellEvent::Ime(event) => self.route_ime(cx, event),
    ShellEvent::Window(_) => {}
}
```

### 4.8 `sigil_hal::Shell` (two new methods, default impls)

```rust
pub trait Shell: Sized {
    // ... existing items ...

    /// Toggle whether the OS IME is allowed for `window_id`. Drives
    /// focus-change-based enable/disable; called by the application
    /// when focus enters/leaves an IME-targeting widget.
    ///
    /// # Errors
    ///
    /// Returns an error when the platform reports IME toggle failure
    /// (rare; most adapters can't fail).
    fn set_ime_allowed(
        &mut self,
        window: &Self::Window,
        allowed: bool,
    ) -> Result<(), Self::Error> {
        let _ = (window, allowed);
        Ok(())
    }

    /// Forward the candidate-window position rect to the OS IME.
    /// `rect` is window-relative physical pixels.
    ///
    /// # Errors
    ///
    /// Returns an error when the platform rejects the rect.
    fn set_ime_cursor_area(
        &mut self,
        window: &Self::Window,
        rect: PhysicalRect,
    ) -> Result<(), Self::Error> {
        let _ = (window, rect);
        Ok(())
    }
}
```

Default impls are no-ops so that test shells and headless shells do
not need to provide implementations. The `WinitShell` impl
delegates to `winit::window::Window::set_ime_allowed` and
`set_ime_cursor_area` respectively.

### 4.9 `sigil_kernel::ImeInputTarget` (widget-builder marker trait)

```rust
/// Marker trait implemented by widget builders that participate in
/// IME composition.
///
/// Mirrors [`Styled`] (DR-049) — a builder-level trait whose only
/// purpose is to expose the composition entity to callers and
/// gating consumers (focus-change dispatch, AccessKit projection
/// setup).
pub trait ImeInputTarget {
    fn composition(&self) -> &Entity<Composition>;
    fn ime_enabled(&self) -> bool { true }
}
```

Lives in `sigil-kernel` because both widget builders and the events
crate reference it (events crate reads it to decide whether to call
`set_ime_allowed(true)` on focus change).

### 4.10 `sigil_events::RecordingImeBackend` (test surface)

```rust
#[derive(Debug, Default)]
pub struct RecordingImeBackend {
    pub dispatched: Vec<(NodeId, ImeEvent)>,
    pub published_rects: Vec<(WindowId, PhysicalRect)>,
    pub allowed_toggles: Vec<(WindowId, bool)>,
}

impl RecordingImeBackend {
    #[must_use]
    pub fn new() -> Self { Self::default() }

    pub fn clear(&mut self);

    #[must_use]
    pub fn last_dispatched(&self) -> Option<&(NodeId, ImeEvent)>;

    #[must_use]
    pub fn last_rect(&self) -> Option<&(WindowId, PhysicalRect)>;
}
```

Lives behind `#[cfg(any(test, feature = "test-support"))]` in the
events crate. Production code never constructs it.

---

## 5. Implementation Phases

### Phase 1 — W4.G7-impl ticket (this DR's implementation scope)

**Step 1: Kernel types + extensions.**
Land `sigil_kernel::ime::Composition`, `ImeTargetExt`,
`CaretAnchorExt`, `ImeInputTarget` trait, and the three
`NodeBuilder` methods (`ime_target`, builder-only — handler
installations land in step 2).
Verify: `cargo test -p sigil-kernel --tests`.

**Step 2: Events crate — handlers + router + builder ext.**
Land `ImeHandlers` extension, `NodeImeExt::on_ime_commit` /
`on_ime_preedit`, `EventRouter::route_ime`, `ime_caret_rect`,
and the new `ShellEvent::Ime` arm in `route`. Land
`RecordingImeBackend` under `#[cfg(test)]`.
Verify: `cargo test -p sigil-events --tests` — including new
unit tests `route_ime_writes_composition`,
`route_ime_invokes_commit_handler`, `route_ime_dropped_when_no_focus`.

**Step 3: Shell — `set_ime_allowed` + `set_ime_cursor_area`.**
Add trait methods to `sigil_hal::Shell` (default impls = Ok no-op).
Implement on `WinitShell` (delegate to winit). Wire app-loop:
on focus change, if new node is `ImeInputTarget`, call
`set_ime_allowed(true)`; otherwise `set_ime_allowed(false)`.
At end-of-Layout, call `EventRouter::ime_caret_rect`; if `Some`,
call `set_ime_cursor_area`.
Verify: `cargo test -p sigil-shell --tests` + smoke test in
`examples/vertical-slice` (open window, focus text_field, verify
no panic on Windows + Linux + macOS via CI matrix).

**Step 4: AccessKit projection.**
In `sigil-a11y::observer`, install an entity observer on every
`ImeTargetExt::composition` at node-attach time. Observer effect:
mark node dirty in `a11y_dirty` + push to `live_announcements`
whenever `composition.preedit` or `commit_seq` changes. In
`mapping::project_node`, when projecting an IME-target node, splice
preedit into `set_value`.
Verify: `cargo test -p sigil-a11y --tests` — new
`ime_preedit_projects_to_live_region` test asserts the
`TreeUpdate::nodes` published at end-of-Effect contains the
composed string.

**Step 5: `text_field` v2 — IME-aware.**
Update `TextFieldBuilder` to construct `Entity<Composition>` on
build, attach `.ime_target(comp)` to the root container, implement
`ImeInputTarget`, register a default `on_ime_commit` handler that
calls `TextEdit::insert_str`, register an `on_ime_preedit` handler
that publishes `CaretAnchorExt` (the rect around the current caret
column). Preedit *rendering* is in §6 phase 2 — phase 1 just routes
text; the preedit overlay can ship as a stub that the visual review
seat refines.
Verify: `cargo test -p sigil --test ime_text_field` —
five golden tests (one per canned platform fixture).

**Step 6: Phase-1 platform fixtures.**
Capture canned `ImeEvent` sequences on each of Windows / macOS / Linux
(IBus + Fcitx5 separately) using a small capture binary in
`examples/ime-capture/`. Land the fixtures at
`tests/fixtures/ime/*.json`. Golden tests in step 5 reference these.
Verify: CI matrix runs `cargo test -p sigil --test ime_text_field`
on all three platforms; all green.

### Phase 2 — Deferred to a future DR (not this ticket)

- Custom IME candidate-window UI (GPUI-style suggestion popover).
- IDE-side composition-aware autocomplete (`sigil-ide-core` consumer).
- Code-editor widget IME integration (multi-line composition,
  syntax-aware commit validation).
- Composition-state inspection tools for the
  `sigil-ide-core::keymap` resolver (debug overlay showing live
  preedit + cursor).

---

## 6. Tests

The test seam from §Decision 7 + the Phase-1 steps above. The
catalogue of mandatory tests:

| Test name                                          | Crate               | Asserts                                                      |
| -------------------------------------------------- | ------------------- | ------------------------------------------------------------ |
| `composition_default_inactive`                     | `sigil-kernel`      | `Composition::default().active() == false`                   |
| `composition_just_committed_detects_seq_bump`      | `sigil-kernel`      | `just_committed(0)` returns true after one commit            |
| `route_ime_writes_composition`                     | `sigil-events`      | `Preedit` event mutates `Entity<Composition>` state          |
| `route_ime_invokes_commit_handler`                 | `sigil-events`      | `Commit` event invokes registered `on_ime_commit`            |
| `route_ime_dropped_when_no_focus`                  | `sigil-events`      | No focus → router drops; recording backend empty             |
| `route_ime_dropped_when_no_ime_target`             | `sigil-events`      | Focused node lacks `ImeTargetExt` → router drops             |
| `route_ime_focus_change_finalizes`                 | `sigil-events`      | Focus loss mid-composition → `Disabled` clears preedit       |
| `ime_caret_rect_returns_layout_relative`           | `sigil-events`      | Rect is window-relative physical, scale-applied              |
| `ime_caret_rect_none_without_anchor`               | `sigil-events`      | Missing `CaretAnchorExt` → `None`                            |
| `shell_set_ime_allowed_default_ok`                 | `sigil-hal`         | Default impl returns `Ok(())`                                |
| `ime_preedit_projects_to_live_region`              | `sigil-a11y`        | Preedit change → next `TreeUpdate::nodes` contains preedit   |
| `ime_commit_does_not_double_announce`              | `sigil-a11y`        | Commit announces via buffer signal, not Live region          |
| `text_field_inserts_committed_text`                | `sigil`             | Canned `Enabled → Preedit → Commit → Disabled` → buffer holds commit |
| `text_field_japanese_kana_fixture`                 | `sigil`             | Golden against `tests/fixtures/ime/japanese_kana.json`       |
| `text_field_korean_hangul_fixture`                 | `sigil`             | Golden against `tests/fixtures/ime/korean_hangul.json`       |
| `text_field_chinese_pinyin_fixture`                | `sigil`             | Golden against `tests/fixtures/ime/chinese_pinyin.json`      |
| `text_field_emoji_picker_fixture`                  | `sigil`             | Golden against `tests/fixtures/ime/emoji_picker_macos.json`  |
| `text_field_fcitx5_smoke`                          | `sigil`             | Golden against `tests/fixtures/ime/fcitx5_linux.json`        |

The fixture files are the load-bearing artefact for the
"complete-on-all-three" gate. CI on all three OSes replays the
fixtures captured on that OS plus the fixtures captured on the
others (a Windows-captured Japanese kana fixture replayed on macOS
must produce identical widget state — winit's translation layer
makes this an industry-standard cross-platform equivalence test).

---

## 7. Consequences

- **No new crate.** All work lands in existing crates: `sigil-kernel`
  gets the `ime` module, `sigil-events` gets the router method +
  `ImeHandlers` + `RecordingImeBackend`, `sigil-hal::Shell` gets two
  trait methods with default impls, `sigil-a11y` gets one observer
  install, `sigil-shell` implements the two new `Shell` methods over
  winit, `sigil`'s `text_field` widget gets IME wiring. No
  `crates/ime/` directory.
- **One new kernel module (`crates/kernel/src/ime.rs`)** holding
  `Composition` (~80 LoC), `ImeTargetExt`, `CaretAnchorExt`,
  `ImeInputTarget` trait. Module budget 200 LoC, hard-split at 350
  per the file-cap rule.
- **`EventRouter` grows one match arm + one method.** The router
  file (`crates/events/src/router.rs`) is currently 174 lines; the
  new `route_ime` + `ime_caret_rect` add ~60 lines. Stays well
  under the 400-line cap.
- **`Shell` trait grows by two methods.** Both have default impls.
  No existing `Shell` implementor (`WinitShell`, headless test
  shells) breaks. The winit impl gains ~20 lines.
- **`RuntimeState` does not grow.** The composition entity lives in
  the existing reactive arena; no new field on `RuntimeState`. The
  `a11y_dirty` set already exists per DR-050 §4.
- **`text_field` widget gains ~50 lines.** The composition entity,
  the `.ime_target(comp)` attachment, the default commit + preedit
  handlers, and the `CaretAnchorExt` publish. Visual rendering of
  preedit (underline overlay) is W4.G7-impl phase 2 and is allowed
  to grow `text_field/render.rs` by another ~50 lines.
- **AccessKit `Live::Polite` becomes the default for every IME
  target.** Widgets can override via `.aria_live(Live::Assertive)`
  but the default is correct for every Phase-1 widget (text_field,
  later text_area, code editor).
- **One new test fixture directory (`tests/fixtures/ime/`)** holds
  per-platform canned `ImeEvent` sequences as JSON. Capture binary
  ships at `examples/ime-capture/` for future re-capture (if a
  Phase-1 fixture goes stale across an OS minor version).
- **The `text_field` event-handler interface changes from
  `on_key_any` to `on_ime_commit`.** Existing `on_key_any` calls
  (used today for printable text insertion) are deprecated for
  text-bearing widgets — the IME path handles `Commit`, and
  non-IME printable input still arrives via `KeyEvent::text` on
  platforms where IME is inactive (e.g. ASCII typing on US keyboard
  layout with no IME engaged). DR-027's `on_key_any` discipline
  stays; `text_field` v2 simply prefers IME-aware ingestion.
- **Phase-1 completeness gate is a CI matrix gate.** A widget that
  ships without all five fixture goldens (kana / hangul / pinyin /
  emoji / fcitx5) cannot mark `cargo test -p sigil --test
  ime_text_field` green. The W4.G7-impl ticket includes the capture
  step.
- **The dispatch-discipline rule applies.** Any platform-specific
  bug surfaced during W4.G7-impl is fixed in `sigil-shell`'s winit
  adapter (workaround at the lowest crate), never in widget code.
  The principle from DR-050 §1 — "one consumer crate owns the
  indirection" — applies identically.

---

## 8. Alternatives Considered

### New `sigil-ime` crate

Rejected. Composition is one match arm on the router, one extension,
one trait, two `Shell` methods, and one observer install. The
crate-fraction-of-LoC heuristic (a crate exists when it owns a
significant piece of policy + an independent SemVer surface) does
not apply: the policy is "route events to the focused target" which
is the events crate's existing job, and the SemVer surface is one
public type (`Composition`) which fits comfortably inside
`sigil-kernel::ime`. Adding a crate for ~150 LoC adds Cargo
overhead, doc-build overhead, and one more `path = ...` workspace
entry without buying any seam.

### Callback-only IME (no `Composition` state, just handlers)

Rejected. The AccessKit projection needs to read current preedit
state at end-of-Effect drain time; with callback-only registration,
the framework would have to maintain shadow state on the side, and
two widgets focusing in sequence would have to synchronize the
shadow state. The reactive `Entity<Composition>` is the substrate
that already solves this; using it is free.

### `Composition` as a `Signal<ImeState>` enum

Rejected (mirroring DR-046 §3 / DR-050 §closing). Composition has
five tightly-coupled fields that mutate in lockstep; modelling them
as one enum (`Idle | Composing { preedit, cursor } | Committed
{ text }`) loses the active-flag persistence across Enabled →
Preedit transitions and forces a clone-and-rebuild on every preedit
keystroke. The struct shape is the right shape.

### Global `current_composition: Option<Entity<Composition>>` on `RuntimeState`

Rejected. Two IME targets in different windows of the same app
(multi-window IDE) need independent composition state. A global
holds at most one and forces a synchronisation question
(if window A is composing, does window B's IME target get
`Disabled`?) that doesn't exist with per-target entities. winit
emits `Ime` events per window; the routing reads focus per window;
each IME target owns its own composition.

### Direct winit `set_ime_purpose(Purpose::Password)` API

Rejected for Phase 1. Password masking during IME is a Wave 5 widget
concern (the password input widget), not a substrate concern. The
field is already present on winit; widgets can call it through a
future `password_input` widget API; G7 doesn't need to mediate.

### Send IME events to *all* IME-targeting nodes in the focus path

Rejected. IME is a single-focused-target affair on every platform.
Bubbling composition events up the focus path would let a parent
intercept (and break) a child's composition, which is exactly the
opposite of the spec the OS implements. One focused leaf, one
composition stream.

### Combine `ImeHandlers` into the existing `EventHandlers` god struct

Rejected per the staff-engineer hygiene contract. `EventHandlers`
already has 13 fields and a clear split-axis at IME — pointer + key
+ scroll + focus are spatial-input concerns dispatched by hit-test
or focus path; IME is composition-state delivery to a single
focused node. Different concern, different storage. The router
reads both extensions; that's the only coupling.

### Phase-1 completeness verified via cargo test on a single OS

Rejected. "Complete-on-all-three" without a CI matrix is a
marketing claim. The fixture-replay test catalogue is precisely
designed so each platform's CI runner replays both its own captures
and the other platforms' captures, asserting equivalent widget
state across all three. This is the industry-standard
cross-platform-equivalence test pattern (cf. `feedback_industry_
standard_verification`).

### Defer the AccessKit `Live::Polite` projection to a future DR

Rejected. WCAG 4.1.3 names status-message announcements as
load-bearing for AA conformance (cited in DR-050 §9). A blind user
composing CJK input *must* hear the preedit text; skipping the
projection means sigil ships a known WCAG 4.1.3 regression for the
exact users IME serves most heavily. The projection is one observer
install (~10 lines in `sigil-a11y`); shipping G7 without it is not
defensible.

---

## 9. Open Questions

Explicitly not resolved here. These land in follow-up DRs or
W4.G7-impl implementation discovery.

1. **Pre-composition input on US keyboard layouts.** On
   Windows + US-English keyboard with no IME, typing 'a' produces
   only `KeyEvent { text: Some("a") }` and no `ImeEvent`. The
   current `on_key_any` path handles this. When a Japanese keyboard
   layout is active but in direct-input mode, the same happens.
   Should the `text_field` always prefer `on_ime_commit` and ignore
   `KeyEvent::text` when an `Entity<Composition>` is attached, or
   keep both paths? Provisional: keep both. `KeyEvent::text` is
   suppressed by the OS when IME is composing (winit's translation
   guarantees this), so no double-insertion. Lock at first
   integration test that proves the two paths don't conflict.

2. **Drag-and-drop and IME interaction.** G8 (DnD) is the next
   wave's substrate; DnD-during-composition is rare but possible
   (user pastes a candidate selection via drag). Provisional: DnD
   commits composition before delivering drop event. Re-evaluate
   in G8 design.

3. **Compose-key sequences on Linux.** XCompose sequences
   (`Compose, ', e → é`) flow through winit as a single
   `KeyEvent::text` after the compose engine finalizes, not through
   `ImeEvent`. No action needed in G7; documented here so a future
   reader doesn't mistake "no `Composition` activity for compose
   keys" as a bug.

4. **Soft-keyboard support on touch devices.** winit's soft-keyboard
   path on Android / iOS (when those platforms are added to the
   support matrix; Phase 1 is desktop-only) delivers text via
   `Ime::Commit` exclusively (no Preedit). Phase-1 design works
   unchanged. The mobile-platform DR (post-Wave-5) will revisit
   `set_ime_cursor_area` semantics (mobile keyboards have no
   candidate window).

5. **CapsLock + Ctrl-key shortcuts during composition.** Some IMEs
   eat Ctrl-A while composing (candidate select-all); others
   pass through to the application. winit normalises but not
   uniformly across IBus versions. Provisional: route key events
   to the focused node *during* composition; widgets that want to
   suppress Ctrl shortcuts during composition check
   `composition.active()` in their `on_key` handler. Document
   the pattern in the `text_field` widget docs.

6. **Multi-line preedit (text area).** Some IMEs (macOS Japanese
   kana, complex Chinese pinyin with disambiguation overlay)
   produce preedit that visually exceeds one line. The Phase-1
   `CaretAnchorExt` is a single rect; multi-line preedit needs
   per-line anchor information. Provisional: lock a
   `CaretAnchorExt::trailing_rect: Option<LogicalRect>` field in
   Phase 2 when the text-area widget hits the case. Phase 1 ignores
   trailing rect; the OS positions the candidate window from the
   leading rect alone, which is correct for single-line text fields.

7. **Custom IME for offline / kiosk apps.** A future application
   may want to embed its own IME (emoji picker, custom keyboard
   layout). The `ImeInputTarget` trait is open enough to support
   this — a widget can implement it and route its own composition
   events through its `Entity<Composition>` without OS
   involvement. Phase 1 doesn't gate this; the substrate is
   composition-source-agnostic.

8. **DR-042 status.** The parity-integration plan references
   "DR-042 driven" for IME. DR-042 does not exist in
   `docs/decisions/` (verified by `Glob DR-042*` returning empty).
   DR-051 is therefore the canonical IME DR; if a draft DR-042
   surfaces later it is superseded.

---

## 10. References

- **DR-002** — `sigil-shell` + winit ownership of OS event delivery.
  The IME ingestion path enters at the shell boundary; this DR
  builds on DR-002's "Shell owns platform delivery; routing /
  hit-testing / focus / handler dispatch live in sigil-events"
  separation.
- **DR-027** — Text input contract. Explicitly deferred IME to a
  future DR. DR-051 is that DR; `TextEdit` (still crate-private)
  absorbs `Commit` text via `insert_str`. DR-027's `on_key_any`
  helper stays for non-IME printable input.
- **DR-045** — `Entity<T>` + `Context<'_, T>`. The reactive
  container `Composition` lives in.
- **DR-046** — `FocusHandle` + entity focus. The router reads
  `FocusManager::focus()` to find the IME target.
- **DR-047** — DrawPhase. Phase ordering for composition writes
  (Effect), caret-rect publish (end-of-Layout), AccessKit drain
  (end-of-Effect).
- **DR-049** — `Styled` fluent trait pattern. `ImeInputTarget`
  mirrors `Styled` as a builder-level marker trait.
- **DR-050** — AccessKit bridge + Live-region drain timing. G7's
  preedit projection rides DR-050 §8's exact drain seam. The
  `RecordingImeBackend` mirrors `crates/a11y/src/bridge.rs:23`
  `RecordingBridge`.
- **`crates/hal/src/events.rs:196-207`** — existing `ImeEvent` enum.
- **`crates/shell/src/shell/events.rs:84-93`** — winit `Ime` →
  `ImeEvent` translation already in place.
- **`crates/events/src/router.rs:61`** — the floor where IME
  currently dies. G7's `route_ime` arm replaces the drop.
- **`crates/events/src/handlers.rs:15-32`** — `EventHandlers`.
  G7's `ImeHandlers` is the parallel extension.
- **`crates/sigil/src/widgets/text_edit.rs`** — `TextEdit` buffer;
  Commit text flows in via `insert_str`.
- **winit 0.30 `Ime` event** —
  https://docs.rs/winit/0.30/winit/event/enum.Ime.html. Source of
  truth for the four variants Phase-1 handles.
- **winit 0.30 `Window::set_ime_allowed`** —
  https://docs.rs/winit/0.30/winit/window/struct.Window.html#method.set_ime_allowed.
- **winit 0.30 `Window::set_ime_cursor_area`** —
  https://docs.rs/winit/0.30/winit/window/struct.Window.html#method.set_ime_cursor_area.
- **WCAG 2.2 §4.1.3 Status Messages** —
  https://www.w3.org/TR/WCAG22/#status-messages. The conformance
  criterion the AccessKit `Live::Polite` projection satisfies for
  IME composition.
- **`docs/architecture/authoring-syntax.md`** — naming contract
  enforced for `.ime_target` / `.on_ime_commit` /
  `.on_ime_preedit` (one-word verbs, no `with_`,
  configure-as-noun).
- **API sketch** — `docs/specs/g7-ime-api-sketch.md` (companion
  to this DR).
