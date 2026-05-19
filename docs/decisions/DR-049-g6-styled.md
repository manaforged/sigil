---
id: DR-049
status: proposed
date: 2026-05-18
covers: G6 `Styled` fluent trait — unified per-builder styling surface across NodeBuilder + every widget builder
depends_on: [DR-040, DR-045, DR-046, DR-047, DR-048]
relates: docs/plans/2026-05-18-parallel-dispatch-master.md, docs/specs/g6-styled-api-sketch.md
---

# DR-049 — G6 `Styled` Fluent Trait: Unified Per-Builder Styling Surface

**Status:** Proposed
**Date:** 2026-05-18
**Covers:** The W4.G6 keystone — a single `Styled` trait that every node
builder and every widget builder implements, providing GPUI-shaped fluent
styling (`button("Save").bg("surface").rounded(6).hover(|s| s.bg("surface-hover"))`)
without per-widget API drift.
**Depends on:** DR-040 (thesis lock — mirror GPUI's validated substrate),
DR-045 (`Entity<T>` + `Context<'_, T>` — builders compose into entities),
DR-046 (action handlers + focus — interaction surface that consumes
styled builders), DR-047 (DrawPhase — atoms flow through Effect-phase
binding install), DR-048 (G5 `Task<T>` — proves the "thin trait over
existing crate substrate" shape this DR also uses).
**Unlocks:** W4.G7 (IME), W4.G8 (DnD), W4.G9 (animation) — each
extends widget builders that inherit `Styled` for free. Wave 5 widget
catalog (~21 widgets) — uniform fluent surface across the catalog.
Public-API ergonomics for downstream consumers (the host engine, third-party
widget crates).

---

## Context

GPUI's headline ergonomic surface is uniform fluent styling: every
element, every container, every layout primitive exposes the same
chainable methods. The shape is grammatical, not type-erased — the
compiler sees a concrete builder type all the way down.

```rust
div()
    .flex().flex_row()
    .gap_2().px_4().py_2()
    .bg(rgb(0x1F2937))
    .text_color(rgb(0xF9FAFB))
    .rounded_md()
    .hover(|s| s.bg(rgb(0x374151)))
```

Three properties make this work in GPUI:

1. **Trait-based default impls.** `Styled` is a trait; every method has
   a default body that pushes a style atom onto a sub-record returned
   by one required method. Adding a new method does not require
   touching every implementor.
2. **No allocation per call.** Method calls push into a `SmallVec`-backed
   atom list that lives on the stack of small style records and only
   spills under unusual atom counts.
3. **State-variant closures.** `.hover(|s| ...)`, `.focus(|s| ...)`,
   `.active(|s| ...)` take a closure that mutates a sub-record for
   that interactive state — same fluent vocabulary recursively, no
   second method family.

### What sigil already has

The atomic and state-record substrate is **already built**. G6 does
not invent any storage; it only adds the unified entry-point shape.

- **`crates/style/src/atom.rs`** — `StyleAtom` enum with 35+ atomic
  style operations (padding/margin/gap, sizing, flex, color via
  `ColorAtom`+`BackgroundThemeRef`, typography, border-radius,
  opacity, overflow, transition). This vocabulary is **locked**;
  G6 only spends from it.
- **`crates/style/src/struct_api.rs`** — `Style` struct with chainable
  methods that push atoms onto a `SmallVec<[StyleAtom; 16]>`. Methods
  consume + return `Self` (`fn padding(self, px: i16) -> Self`). The
  full vocabulary already maps to atoms one-for-one.
- **`crates/style/src/state.rs`** — `StyleState` (interaction flags),
  `StateSlot` (priority resolver), `StateStyle` (five `&'static
  [StyleAtom]` slot overlays: rest/focused/hovered/pressed/disabled),
  `StyleVariant` (base atoms + state overlays). Designed for `const`
  construction so widget crates can place state tables in `static`.
- **`crates/style/src/variant.rs`** — `Variant` (Normal / Terminal
  shape vocabulary). Orthogonal to G6; not touched.
- **`crates/style/src/parser.rs`** — class-string parser
  (`parse_class_string("px-4 py-2 bg-accent")`).
- **`crates/style/src/builder_ext.rs`** — `NodeStyleExt` trait with
  five methods (`class`, `style`, `theme`, `focus_ring`,
  `style_reactive`) implemented for `NodeBuilder` + `TextBuilder`.
  `ClassSource` enum unifies static slices, owned `Style`, and raw
  class strings under one entry point.
- **`crates/style/src/reactive.rs`** — `StyleReactiveEffect`. Used by
  `NodeStyleExt::style_reactive` for signal-driven atom installation.

Every widget builder lives in `crates/sigil/src/widgets/*`. Each
defines its own ad-hoc surface:

- `ButtonBuilder` (`button.rs:114-132`) — `.class()`, `.style()`,
  `.theme()` each implemented by hand, each pushing a `BuilderMutator`
  closure onto `self.mutators`. Three near-identical methods per
  widget that exist only because there's no shared trait.
- `CheckboxBuilder` (`checkbox.rs:120-138`) — same three methods,
  copy-pasted.
- `PanelBuilder` (`panel.rs:7-21`) — different shape: stores style
  config as struct fields (`padding: f32`, `elevation: u32`)
  rather than a `Style` accumulator. Some widgets do one thing, some
  another. There is no consistent surface.
- `DropdownBuilder`, `SliderBuilder`, `ToggleBuilder`,
  `TextFieldBuilder`, etc. — each repeats the `.class()` / `.style()`
  / `.theme()` trio and adds widget-specific tuning methods.

### What does not exist

- **No unified entry point.** A consumer writing
  `button("Save").bg("surface")` gets a method-not-found error today.
  The only existing fluent surface on widgets is `.class("bg-surface")`
  (class-string) or `.style(Style::new().background_theme("surface"))`
  (struct-style). Neither matches the GPUI shape — neither lets
  authors say `.bg(...)` directly on the widget builder.
- **No state-variant closure.** Widgets that need hover/focus/pressed
  overlays declare them inline as `static [StyleAtom]` slices plus a
  `style_reactive` callback that resolves the variant on each frame
  (see `button.rs:33-46`, `checkbox.rs:36-69`). This is the
  correct **kernel-level** mechanism; G6 layers an **author-level**
  fluent surface on top so widget authors can spell hover atoms
  inline without authoring static slices by hand.
- **No trait coherence.** The widget-specific `.class()` / `.style()`
  / `.theme()` methods are not part of a trait. Adding a new style
  affordance (say, `.opacity(u16)`) requires editing every widget
  builder. With G6, the new method lives once on the trait.

### Why a trait is the right shape

Three alternatives were considered (formally rejected below):

1. **Inherent methods on each widget builder.** Status quo. Linear
   maintenance cost in widget count × method count. Wave 5 adds 21
   widgets; the cost compounds.
2. **Free-standing functions taking `&mut Style`.** `bg(&mut style,
   "surface")`. Loses chaining. Not GPUI-shaped.
3. **`Styled` trait with default impls + one required method.** GPUI's
   shape. Linear in `Styled` method count, constant in widget count.
   The compile-time-dispatch property survives because every call site
   sees a concrete `impl Styled` type.

DR-040 commits sigil to mirroring GPUI's validated substrate; (3) is
the structural mirror.

---

## Decision

Ten decisions. Each is locked for the W4.G6-impl dispatch.

### 1. One trait, one required method, default impls for the rest

```rust
pub trait Styled: Sized {
    /// The single required method. Returns a mutable borrow of the
    /// implementor's owned `Style` accumulator. All other methods
    /// default-impl via this.
    fn style(&mut self) -> &mut Style;

    // All other methods (~50) have default bodies that mutate
    // `self.style()` then return `self`. Implementors override
    // only if they need widget-specific interception.
}
```

`Style` is the existing `crates/style/src/struct_api.rs::Style` —
already a `SmallVec<[StyleAtom; 16]>` accumulator with the right atom
vocabulary. Builders own a `Style` field; the trait's required method
returns `&mut self.style`. Every chainable method consumes
`self`, mutates the accumulator, returns `self`. The trait is `Sized`
because the consume-and-return signature requires it.

**The trait's accumulator is the existing `Style` struct.** No new
type. No new atom enum. The reactive-binding install
(`NodeStyleExt::style`, which today calls `install_class` → attaches
a `StyleBinding`) is reused unchanged for `NodeBuilder` and widget-
builder root nodes.

**Why one method, not zero:** zero methods (a marker trait + helper
functions) loses self-mutation discoverability — `cx.style()` would
need to look up where to push. One method is the GPUI shape and the
minimal surface that gives every default impl a place to write.

**Why not a separated trait for "leaf widgets" vs "containers":** GPUI
unifies them; the atom vocabulary is the same; the storage is the
same. A split exists only to forbid `align_items` on a leaf, which is
a contract not worth a type-level split (the layout engine ignores
flex atoms on non-flex parents already).

### 2. Method naming follows the existing struct-API + authoring-syntax doctrine

Methods are **one word** (`.bg()`, `.gap()`, `.opacity()`,
`.rounded()`), never `with_` prefixed. Methods take `self` and
return `Self` for chaining. Naming is identical to the existing
`struct_api::Style` methods so the two surfaces are interchangeable —
the only delta is what the builder owns versus what the trait borrows.

The full G6 vocabulary mirrors `Style`'s today plus two GPUI-shaped
additions:

- **Spacing:** `padding`, `px`, `py`, `pt`, `pr`, `pb`, `pl`,
  `margin`, `mx`, `my`, `mt`, `mr`, `mb`, `ml`, `gap`.
- **Sizing:** `w`, `h`, `min_w`, `max_w`, `w_full`, `h_full`,
  `w_auto`, `h_auto` (the existing `width(Dimension)` becomes the
  generic; `w_full` etc. are convenience wrappers like
  `width(Dimension::Full)`).
- **Display + flex:** `flex`, `block`, `hidden`, `flex_row`,
  `flex_col`, `flex_row_reverse`, `flex_col_reverse`, `items_start`,
  `items_center`, `items_end`, `items_stretch`, `justify_start`,
  `justify_center`, `justify_end`, `justify_between`,
  `justify_around`, `grow`, `shrink`.
- **Color:** `bg(impl Into<ColorOrToken>)`, `text_color(impl
  Into<ColorOrToken>)`. The `Into<ColorOrToken>` argument accepts
  `Rgba8`, `&'static str` (theme token), `ColorAtom`. One method per
  paint property, polymorphic over input.
- **Typography:** `font_size`, `font_weight`, `font_family`,
  `line_height`.
- **Geometry:** `rounded`, `rounded_sm`, `rounded_md`, `rounded_lg`.
- **Visual:** `opacity` (accepts `u16` per existing encoding, or `f32`
  via the same Into-trick), `overflow_x`, `overflow_y`,
  `overflow_hidden`.
- **Transitions:** `transition(TransitionId)`,
  `transition_duration(MillisU16)`.
- **State variants:** `hover`, `focus`, `pressed`, `disabled` (see
  Decision 4).
- **Escape hatches:** `class(impl Into<ClassSource>)`,
  `style_reactive(impl Fn() -> &'static [StyleAtom] + 'static)`,
  `theme(Theme)`, `focus_ring(FocusRing)` — the existing
  `NodeStyleExt` quartet, lifted onto `Styled` so widget builders
  inherit them.

**Why not GPUI's literal Tailwind names** (`gap_2`, `px_4`, etc., where
the trailing number encodes a fixed spacing step): sigil's existing
struct API uses `gap(px: i16)` with a runtime argument. Matching
GPUI's `gap_2` (compile-time-encoded 2-step = 8 px) would require
either generating ~10 fixed-step variants per spacing method (200+
methods) or duplicating the surface (`gap` + `gap_2`/`gap_4`/etc).
**Rejected:** the runtime-argument shape composes better with theme-
derived spacing scales and matches the existing `struct_api::Style`
vocabulary one-for-one. Authors who want the Tailwind shorthand can
use `.class("gap-2")` — the class-string parser already supports it.

### 3. Compile-time dispatch only — no vtable cost

The trait is **never** consumed as `dyn Styled`. Every call site sees
a concrete `impl Styled` type. The trait's `Self: Sized` bound (from
the consume-and-return signature) makes `dyn Styled` impossible
anyway. There is no dynamic-dispatch path through G6.

This matters because the per-method cost is one SmallVec push (a
single byte-copy in the inline case, ~8 ns on a 5090). A vtable
indirection would add ~3 ns × per-call cache miss; on a hot frame
with 1000 styled atoms applied across the tree, that's 3 µs of
overhead for zero gained capability.

GPUI is also entirely static-dispatch here; we mirror.

### 4. State-variant methods take `FnOnce(StateStyleBuilder) -> StateStyleBuilder`

```rust
pub trait Styled: Sized {
    fn style(&mut self) -> &mut Style;  // required

    // ... ~50 styling methods with default impls ...

    fn hover<F>(mut self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder
    {
        let builder = StateStyleBuilder::new();
        let built = configure(builder);
        self.style().set_hover(built.into_atoms());
        self
    }

    // .focus, .pressed, .disabled — identical shape.
}
```

`StateStyleBuilder` is **a thin wrapper around an owned `Style`**
that exposes the same fluent vocabulary. Calling `.bg("token")` inside
a hover closure pushes the same atom as calling `.bg("token")` on the
outer builder — the only difference is which atom list it lands in.

This is achievable two ways; G6 picks the second:

- **(A) `StateStyleBuilder` is a distinct type that re-implements
  every styling method.** Doubles the surface; bug-prone.
- **(B) `StateStyleBuilder` implements `Styled` itself.** Same vocab
  by inheritance. **Chosen.** The state builder's `style()` method
  returns a mutable borrow of its **own** owned `Style`; the outer
  trait's `style()` method returns the outer builder's.

This is the GPUI shape — `gpui::Styled::hover(self, f: impl FnOnce(Self::Style) -> Self::Style)` —
modulo GPUI's stylistic choice of `s.bg(...)` instead of `s: Self::Style`. We name
the wrapper `StateStyleBuilder` to disambiguate from the existing
`StyleVariant` (the `const`-buildable runtime resolver).

**The state record stores `SmallVec<[StyleAtom; 8]>` per slot.** The
existing `StateStyle` (`crates/style/src/state.rs`) is `&'static
[StyleAtom]` per slot for `const`-constructibility. The fluent-state
record at the builder level is **owned**, not `&'static`. When the
builder commits to the kernel node, it leaks the per-slot SmallVecs
to `&'static` slices via the same `parse_and_leak` path the class-
string cache already uses (`builder_ext.rs:202-205`).

Specifically the trait stores per-slot owned vectors as new fields on
`Style`:

```rust
// In crates/style/src/struct_api.rs — additive change:
pub struct Style {
    atoms: SmallVec<[StyleAtom; 16]>,
    // NEW state-slot accumulators. Default-empty. Set via Styled::
    // hover/focus/pressed/disabled.
    hover_atoms: SmallVec<[StyleAtom; 4]>,
    focus_atoms: SmallVec<[StyleAtom; 4]>,
    pressed_atoms: SmallVec<[StyleAtom; 4]>,
    disabled_atoms: SmallVec<[StyleAtom; 4]>,
}
```

When attached to a node, the binding install (`install_class`) is
extended to also install a `StyleBinding` carrying the state overlay
for whichever slots are non-empty. The runtime resolver
(`StyleEngine`) already consumes `StateStyle`; the seam is "owned
`Style` with state slots → `&'static StateStyle` at install time."

**Why this shape:** mirrors GPUI exactly; the closure receives a thing
that responds to the same fluent vocabulary; the storage matches the
existing `StateStyle` slot shape (5 slots: rest, focus, hover,
pressed, disabled — `rest` is the outer-builder atoms list).

**Why not "give the closure `&mut Style` directly":** GPUI's API gives
the closure a wrapper to discourage state-of-state-of-state recursion
(`s.hover(|s2| s2.hover(...))`). The wrapper is a type-level seal.
We do the same; `StateStyleBuilder` is `Sized` and does not implement
`hover` / `focus` / `pressed` / `disabled` on itself. Compile error
on nested state variants is the structural enforcement.

**Mutual exclusion at the variant level.** Calling `.hover(...)`
twice **overwrites** the hover record. The slots are stored as
separate fields on `Style`, each replaced by a `.hover()` call. This
is the GPUI behavior, documented explicitly so authors don't expect
additive composition across multiple `.hover()` calls. (Within a
single `.hover()` closure, multiple `.bg(...)` calls accumulate; the
outer `.hover()` setter assigns the closure's output to the hover
slot whole-cloth.)

### 5. Class-string interop preserved, composes additively

The trait inherits `.class(impl Into<ClassSource>)` from the existing
`ClassSource` enum (`builder_ext.rs:14-46`). The atom list grows
additively across `.class(...)` and fluent calls:

```rust
container()
    .class("px-4 py-2")     // adds 2 atoms via parser
    .bg("surface")          // adds 1 atom via Styled::bg
    .class("rounded-md")    // adds 1 atom via parser
```

Internally `.bg("surface")` mutates `self.style().atoms`. `.class(...)`
funnels through `install_class`'s existing merge-classes path. The
order matches authoring order — last-wins for conflicting atoms,
mirroring the existing `merge_classes` semantics.

### 6. Reactive style integration — `.style_reactive` lifts onto `Styled`

The existing `style_reactive(impl Fn() -> &'static [StyleAtom] +
'static)` on `NodeStyleExt` lifts onto `Styled` unchanged. Default
impl delegates to the existing install path. Calling `style_reactive`
adds a `StyleReactiveEffect` extension to the node at build time; the
effect fires whenever its underlying signal changes.

`.style_reactive` and fluent methods compose: fluent atoms apply on
mount; reactive atoms layer on per the existing cascade
(`engine.rs::StyleEngine` resolves base → variant → reactive →
state). The cascade order is **not** changed by G6.

### 7. `Styled` is a sibling to `LayoutBuilderExt`, not a replacement

**Layout methods stay on `LayoutBuilderExt`.** `LayoutStyle`
(`crates/layout/src/style.rs`) is the kernel layout's source-of-truth
record; it carries Taffy-mappable values that flow directly into the
layout engine. `Style` is the style atoms that flow into the
`StyleEngine` cascade. The two layers are deliberately separate per
DR-041 (owned layout engine).

The two-trait split is honest about the underlying storage:

- `.layout(LayoutStyle::flex_row().fixed_size(320.0, 80.0))` —
  layout-engine-bound, attaches a `LayoutStyle` extension.
- `.bg("surface").gap(8)` — style-cascade-bound, mutates a `Style`
  accumulator.

**Overlap zone.** Three method families look like they could live on
either trait:

- **`gap`** — `Style::gap(px)` pushes `StyleAtom::Gap(Px(px))`;
  `LayoutStyle::gap_px(f32)` sets the layout-engine gap directly.
  These are **different storage** but the same conceptual value.
  **Resolution:** `Styled::gap(i16)` stays the public fluent surface;
  the style engine's resolver propagates the resolved gap to the
  attached `LayoutStyle` via the existing engine pipeline. Authors
  write `.gap(8)` once; if they want explicit layout control they
  use `.layout(LayoutStyle::...)` and bypass the style cascade for
  that property.
- **`width` / `height`** — same shape as `gap`. `Styled::w(...)`
  pushes a `StyleAtom::Width(...)` that is resolved into
  `LayoutStyle.width` by the engine. `.layout(...)` bypasses if set.
- **`flex` / `flex_row` / `items_*` / `justify_*`** — same shape.

The boundary is: **`Styled` is the high-level fluent ergonomics;
`LayoutBuilderExt` is the precise layout-engine knob.** Both ship.
Authors mostly use `Styled`; advanced layout uses
`LayoutBuilderExt`. This mirrors GPUI's split between `Styled` (the
fluent surface) and `style.layout` (the Taffy struct).

**No flag for "prefer fluent vs prefer layout."** The cascade order is
fixed: explicit `LayoutStyle` extension wins because it lives on the
node directly; resolved style atoms set values only on properties the
`LayoutStyle` extension did not.

### 8. State-method overwrite semantics — last-call-wins per slot

`.hover(...)` called twice replaces the hover slot's atoms with the
second call's output. Same for `.focus`, `.pressed`, `.disabled`.
Within a single `.hover(closure)`, multiple atom-pushing calls in
the closure body accumulate into that slot.

Document the behavior explicitly:

```rust
button("Save")
    .hover(|s| s.bg("a"))     // hover slot = [Bg("a")]
    .hover(|s| s.bg("b"))     // hover slot = [Bg("b")] — overwrites
```

```rust
button("Save")
    .hover(|s| s.bg("a").text_color("on-a"))  // hover slot = [Bg("a"), Text("on-a")]
```

This is GPUI's documented behavior and the boring-correct shape.
Authors who want additive hover composition layer it explicitly
inside a single closure.

**Why not additive across calls:** "merge atoms from the second
`.hover` into the first" requires knowing which atoms supersede
which. Two `Bg` atoms is straightforward; mixed atom merging
becomes a per-pair rule table. Last-write-wins per slot is a single
rule that authors can keep in their head.

### 9. Trait lives at `crates/style/src/styled.rs`; re-export at `lib::prelude`

New module: `crates/style/src/styled.rs`. Hosts:

- `pub trait Styled: Sized` (the trait itself).
- `pub struct StateStyleBuilder` (the closure parameter, implements
  `Styled` recursively).
- `impl Styled for NodeBuilder` (the kernel impl).

Re-exports:

- `crates/style/src/lib.rs` — add `pub mod styled;` and
  `pub use styled::{Styled, StateStyleBuilder};`.
- `crates/style/src/lib.rs::prelude` — re-export `Styled` and
  `StateStyleBuilder` alongside the existing `NodeStyleExt`.

Per-widget impls live **in each widget's crate / module**:

- `crates/sigil/src/widgets/button.rs` — `impl Styled for
  ButtonBuilder` (1 line: `fn style(&mut self) -> &mut Style {
  &mut self.style }`).
- Same one-line impl per widget builder file. Widget builders gain
  a `style: Style` field where they currently use ad-hoc mutators.
- The existing per-widget `.class()` / `.style()` / `.theme()`
  methods are **removed** in W4.G6-impl; they become trait-inherited.

`NodeStyleExt` is **kept** for the install-side mechanics
(`style_reactive`, `focus_ring`, the `ClassSource` machinery the
trait depends on). `Styled` and `NodeStyleExt` are sibling traits;
authors typically import `sigil_style::prelude::*` which carries both.

**Why a new module, not in `struct_api.rs`:** the trait is structurally
distinct from the `Style` struct. Cohabitation in `struct_api.rs`
would push that file past the 400-line cap. The `styled.rs` module
budget is ~250 lines for the trait + default impls; ~50 lines for
`StateStyleBuilder`; ~10 lines for `impl Styled for NodeBuilder`.

### 10. Method coverage — full G6 scope in one DR-impl, phased only if budget forces

Single dispatch ships the full ~50-method vocabulary. The methods are
nearly identical in shape (one or two lines of body each, all
delegating to `self.style().push(...)`), and a phased rollout would
ship a partial surface that authors must work around.

**If W4.G6-impl runs out of token budget**, the phased fallback is:

- **Phase G6.1 — spacing + sizing + display + flex.** ~25 methods.
  The structural styling primitives.
- **Phase G6.2 — color + typography + geometry + visual.** ~15
  methods.
- **Phase G6.3 — state variants + escape hatches.** ~10 methods.
  This is the GPUI-headline piece; ships last only if budget forces.

W4.G6-impl is expected to ship the full vocabulary in one pass. The
phased fallback is the implementer's escape valve, not the plan.

---

## Implementation seams

Files touched by W4.G6-impl, in dispatch order:

1. **`crates/style/src/struct_api.rs`** — add four owned state-slot
   accumulator fields to `Style` (`hover_atoms`, `focus_atoms`,
   `pressed_atoms`, `disabled_atoms`). Add their setters
   (`Style::set_hover(atoms)` etc.), used by the trait's state-variant
   methods. Existing API surface unchanged.
2. **`crates/style/src/styled.rs`** (new) — `Styled` trait + default
   impls + `StateStyleBuilder` + `impl Styled for NodeBuilder`.
   Budget: ~350 lines max (close to the cap; the methods are
   one-liners). Split into `styled.rs` + `styled_state.rs` if it
   exceeds the cap.
3. **`crates/style/src/lib.rs`** — `pub mod styled;` + re-exports.
4. **`crates/style/src/builder_ext.rs`** — extend `install_class` (or
   a new sibling `install_style_with_states`) to attach a
   `StyleBinding` that carries the state-overlay atoms for any
   non-empty slot. The `parse_and_leak` path is the existing model
   for owned→`&'static` lifting; reuse it.
5. **`crates/style/src/engine.rs`** — verify the existing
   `StyleEngine` cascade (base → variant → reactive → state) consumes
   the new state overlays correctly. No structural change expected;
   the state overlay is already the last layer.
6. **`crates/sigil/src/widgets/*.rs`** — for each widget builder:
   - Add `style: Style` field (or replace the existing `mutators:
     Vec<BuilderMutator>` where it's only used for `.class` / `.style`
     / `.theme` plumbing).
   - Add `impl Styled for FooBuilder { fn style(&mut self) -> &mut
     Style { &mut self.style } }` (one line per widget).
   - Remove the now-redundant per-widget `.class()` / `.style()` /
     `.theme()` inherent methods (they become trait-inherited).
   - In `into_node_builder`, attach `self.style` to the root node via
     the existing `NodeStyleExt::style` path.
7. **`crates/sigil/src/widgets/shared.rs`** — the `BuilderMutator`
   type may become obsolete; check call sites before deletion.
   `widget-shared` is a small file; do not expand the diff to clean
   adjacent code that wasn't asked for.

Test surface — `crates/style/tests/styled.rs` (new):

- Default-impl-chains round-trip atoms.
- State-variant closures push to the right slot.
- Hover/focus/pressed/disabled overwrite-on-second-call.
- Class-string composes additively with fluent.
- `NodeBuilder` implements `Styled`.
- One widget (button) end-to-end.

---

## Consequences

- **Public API uniform across builders.** `NodeBuilder` and every
  widget builder respond to the same fluent vocabulary. Wave 5
  inherits this for free; ~21 widgets ship without re-litigating
  the styling surface.
- **One canonical authoring surface.** Authors stop choosing between
  `.class("...")` strings and `.style(Style::new()...)` structs.
  They write `.bg("surface").rounded(6)` directly on the builder.
  Both alternative shapes remain supported as escape hatches.
- **Storage cost: 4 × `SmallVec<[StyleAtom; 4]>` per `Style`.**
  Inline capacity 4 atoms × `sizeof(StyleAtom)` (max variant ~8 B)
  × 4 slots = ~128 B extra per `Style`. Style structs are short-
  lived during builder evaluation; the cost is freed on attach.
  Long-lived storage (the `StyleBinding` extension on the node)
  uses `&'static [StyleAtom]` slices via the existing leak path; no
  long-lived overhead delta vs the current path.
- **Compile-time dispatch retained.** No vtable, no boxing in the
  default-impl path. The trait is `Sized` (consume-return signature
  precludes `dyn Styled`).
- **State-variant closures are sealed.** `StateStyleBuilder`
  implements `Styled` for the inner fluent vocabulary but **does
  not** implement `.hover` / `.focus` / `.pressed` / `.disabled`
  recursively. Nested state variants are a compile error, not a
  runtime surprise.
- **Class-string parser unchanged.** The existing token vocabulary
  (`px-4`, `gap-4`, `rounded-md`, etc.) still works exactly as
  today.
- **`NodeStyleExt` survives.** `style_reactive` and `focus_ring`
  live on it; `Styled` re-exposes them via default-impl delegation
  so call sites see them on the trait.
- **`LayoutBuilderExt` survives unchanged.** The two-trait split is
  honest about the underlying storage (atoms cascade vs Taffy-
  shape `LayoutStyle` extension). Authors compose `.layout(...)`
  and `.bg(...)` freely on the same builder.
- **Wave 5 unblocked.** Every widget in the catalog plan inherits
  the trait via one-line impl. No widget needs to re-author the
  styling surface.
- **Public consumers (the host engine) inherit cleanly.** the host engine's UI dispatch
  produces builders; chaining `.bg("hp-bar-red")` works the moment
  G6 lands. No the host engine-side code change required for the surface.

---

## What this does NOT cover

- **Layout primitives.** `LayoutBuilderExt` is separate; G6 does not
  touch it. The overlap zone (gap / width / flex / items / justify)
  is resolved in Decision 7 — `Styled` provides the fluent surface,
  `LayoutBuilderExt` is the precise knob, the resolver propagates.
- **Theme token resolution.** `bg("surface")` still pushes
  `StyleAtom::BackgroundThemeRef("surface")`. The token-to-color
  resolution happens in `StyleEngine`, unchanged.
- **Animation timing.** `.transition` and `.transition_duration`
  push existing atoms; the actual animation runtime lands in W4.G9.
  G6 does not animate; it stores the intent.
- **Style atoms themselves.** The atom enum is locked at
  `crates/style/src/atom.rs`. G6 does not add new atoms. If a new
  atom is genuinely needed (e.g., `box-shadow`), it lands as a
  separate atom-vocabulary expansion, then G6's method coverage is
  extended.
- **Cross-platform rendering of styled atoms.** The renderer paint
  pipeline is unchanged; it already consumes the existing atom
  vocabulary.
- **CSS-style cascading specificity.** Sigil's resolution is
  positional (last-wins per atom kind) per the existing
  `merge_classes` behavior. G6 does not introduce a specificity
  algorithm.
- **Reactive style closures with `Signal<T>` capture.** The existing
  `style_reactive(|| atoms)` shape is preserved; G6 does not add a
  GPUI-style `signal_bg(some_signal)`-shape per-property reactive
  binding. Open question 3 below covers the future shape.

---

## Open questions

These do **not** block W4.G6-impl. Each has a provisional answer; the
implementer escalates only if the provisional choice trips on the
implementation.

1. **`with_layout(closure)` escape hatch — should it get a fluent
   `Styled`-side equivalent now?** Today `NodeBuilder` exposes
   `.with(LayoutStyle)` and widgets pass `.layout(...)`. The fluent-
   style world might want `.layout(|l| l.flex_row())` as a closure-
   style equivalent. **Provisional:** no — the existing
   `.layout(LayoutStyle::...)` is fluent enough; the closure adds a
   wrapping layer with no clear win. Reopen if widget authors find
   the chained-`LayoutStyle::` shape clunky.

2. **Phased method coverage (G6.1 / G6.2 / G6.3) vs single ship.**
   Decision 10 picks single-ship. **Provisional:** stay with
   single-ship; the methods are nearly trivial bodies and a phased
   ship leaves authors with a half-trait. Reopen only if W4.G6-impl
   reports token budget exhaustion mid-pass.

3. **Per-property reactive binding (`signal_bg(some_signal)`).**
   GPUI has `.when(predicate, |s| s.bg(...))` and the related
   reactive shapes; sigil has `style_reactive(|| atoms)` at the
   coarse-grained level. **Provisional:** leave `style_reactive` as
   the reactive seam; do not add per-property signal bindings in
   G6. Reopen in a follow-up DR if downstream consumers need finer-
   grained reactivity than the whole-atoms-slice path supports.

4. **`Styled::when(cond, |s| ...)` conditional sub-record.** Useful
   for `.when(state.is_loading(), |s| s.opacity(5000))`. **Provisional:**
   defer to a follow-up; it's an ergonomic add, not a structural
   change, and it composes with the existing trait via one extra
   default-impl method. Land in W4.G6-impl if budget allows;
   otherwise queue as a Wave-5-or-later widget-ergonomics task.

5. **Where exactly do leaked state-slot atoms go?** The
   `parse_and_leak` path lives in `builder_ext.rs`; the state-overlay
   commit shares the same machinery. **Provisional:** centralise the
   leak helper in a new `crates/style/src/leak.rs` module if both
   sites need it; otherwise keep the existing path. W4.G6-impl makes
   the call.

6. **Migration shim for widgets currently using `BuilderMutator`.**
   Widget code currently stores `mutators: Vec<BuilderMutator>` and
   applies them at `into_node_builder` time. The G6 shape stores
   `style: Style` and attaches at the same point. **Provisional:**
   keep `BuilderMutator` for non-style mutators (the few widgets
   that need to mutate the kernel node for non-style reasons — e.g.,
   panel's `child(...)` closure). Style mutations go through the
   trait directly. The transition is mechanical per widget.

---

## Alternatives considered

### Inherent methods on each widget builder (status quo extended)

Rejected. Linear maintenance cost in widget count × method count.
Wave 5 adds 21 widgets; every styling method that wants to land
requires editing every widget builder. The status quo already
demonstrates the cost: `.class()` / `.style()` / `.theme()` are
copy-pasted across ~10 widget files today.

### A `StyleAtomBuilder` free-standing builder with `&mut style` taken by every method

Rejected. Loses chaining. Doesn't match GPUI's shape. Authors would
write:

```rust
let mut style = Style::new();
bg(&mut style, "surface");
gap(&mut style, 8);
button("Save").style(style)
```

instead of the desired fluent shape.

### Macro-generated per-widget style trait

Considered. A proc macro could generate `impl Styled` from a marker
attribute on each widget. **Rejected:** the one-line manual impl is
the minimum viable surface; a macro would gain nothing the manual
impl doesn't already give, and would hide the trait surface from grep.

### `Styled` as a marker trait + helper functions

Rejected. Loses self-mutation discoverability and chaining. The trait
contract is "the self method finds the style"; that's a method, not a
marker.

### `Style` accumulator becomes the trait associated type, not a struct field

Rejected. Forces every implementor to declare an `type Style = Style;`
line. Adds zero capability; pure churn.

### `StateStyleBuilder` is a distinct type with its own method vocabulary

Rejected (in favor of the recursive-Styled-impl shape in Decision 4).
A distinct type would double the trait surface — 50 methods on
`Styled`, 50 methods on `StateStyleBuilder`. The recursive impl is the
GPUI shape and the maintenance-light shape.

### State variants ship as one method `state(StyleState, |s| ...)` instead of four

Considered. `.state(StyleState::Hover, |s| s.bg(...))` is more
orthogonal than `.hover(|s| ...)`. **Rejected:** four named methods
are the GPUI shape and the discoverable shape. Authors think "I want
hover styling," not "I want state-variant styling parameterised on
the hover slot."

### State methods additive across calls (merge instead of overwrite)

Rejected (Decision 8 alternative). Mixed-atom merge requires a per-
pair rule table; last-call-wins is one rule. GPUI is last-call-wins;
we mirror.

### `Styled` and `LayoutBuilderExt` collapsed into one trait

Rejected (Decision 7 alternative). The underlying storage is
genuinely different (`Style` atom cascade vs `LayoutStyle` Taffy
struct), and collapsing them would either force all layout methods
through the atom cascade (loses the precision of the layout
extension) or all style methods through the layout extension (loses
the cascade composition). Two traits, sibling impls, is honest
about the storage.

### `Styled::class` accepts `&str` instead of `impl Into<ClassSource>`

Rejected. The existing `ClassSource` enum is the polymorphic entry
point for static slices / owned `Style` / raw class strings. Trait
re-exposure must preserve the polymorphism or callers regress.

### Tailwind-step naming (`gap_2`, `px_4`)

Rejected (Decision 2 alternative). Would require either 10+ fixed-
step methods per spacing primitive (200+ trait methods, unmaintainable)
or a duplicate surface (`gap` + `gap_2`/`gap_4`/etc.). The runtime-
argument shape composes better with theme-derived spacing scales and
matches the existing `struct_api::Style` vocabulary one-for-one.

### `Styled` lives in `sigil-kernel` so kernel-level builders can implement it directly

Rejected. The trait depends on `Style` which lives in `sigil-style`.
Moving `Style` into kernel inverts the existing dependency
(`sigil-style` depends on `sigil-kernel`, not the other way). The
kernel-side impl (`impl Styled for NodeBuilder`) lands in
`sigil-style` because that's where the trait lives; the impl borrows
the kernel builder as a sibling crate dep, which sigil-style already
has.

### Move `Style` itself into the kernel to enable the kernel-side impl in the kernel crate

Rejected. `Style` is style-domain semantics. The kernel crate is
structural-node semantics. Crossing the boundary for the sole purpose
of co-locating one impl is the wrong trade.

---

## References

- **DR-040** — sigil thesis lock (mirror GPUI's validated substrate).
- **DR-041** — owned layout engine (the `LayoutStyle` side of the
  Decision 7 split).
- **DR-045** — G2 entity model (entities consume widget builders; the
  styled builder lands as part of the entity's view tree).
- **DR-046** — G3 actions + focus (interaction-state plumbing that
  drives the `StyleState` resolver consumed by G6 state overlays).
- **DR-047** — G4 DrawPhase (style atoms install during Effect; the
  paint pass reads `RenderStyle` from the engine — no Paint-phase
  mutation through G6).
- **DR-048** — G5 `Task<T>` (the "thin trait/handle layered atop
  existing crate substrate" precedent G6 also follows).
- **`crates/style/src/struct_api.rs`** — the existing `Style`
  vocabulary G6 lifts onto a trait.
- **`crates/style/src/state.rs`** — the existing `StyleState` /
  `StateStyle` / `StyleVariant` machinery G6 reuses for state slot
  storage.
- **`crates/style/src/builder_ext.rs`** — the existing `NodeStyleExt`
  trait, the install paths (`install_class`, `install_reactive`),
  and the `parse_and_leak` helper G6 reuses.
- **`crates/style/src/engine.rs`** — the cascade resolver
  (`StyleEngine`) that consumes the atom output of G6.
- **`crates/sigil/src/widgets/button.rs`** — the widget that drives
  the per-widget impl shape (one-line impl + replace
  `BuilderMutator` style mutators with `Style` field).
- **`crates/sigil/src/widgets/checkbox.rs`** — second exemplar with
  state-overlay machinery (`static [StyleAtom]` slot tables) that
  G6 state-variant closures replace at the author level.
- **Master dispatch plan** —
  `docs/plans/2026-05-18-parallel-dispatch-master.md` (W4.1 / W4.G6
  is this DR; W4.G6-impl is the implementation).
- **API sketch** — `docs/specs/g6-styled-api-sketch.md` (companion to
  this DR; pseudocode + test outline).
- **GPUI reference** — `zed-industries/zed/crates/gpui/src/styled.rs`
  (the `Styled` trait sigil mirrors).
