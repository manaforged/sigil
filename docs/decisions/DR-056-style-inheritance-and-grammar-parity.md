---
id: DR-056
status: accepted
date: 2026-07-16
covers: Typography inheritance, class-grammar/atom parity, no-silent-no-op token contract, documented precedence ladder
depends_on: [DR-006, DR-018, DR-049]
relates: docs/plans/2026-07-16-production-readiness.md
---

# DR-056 — Style-system design completion: inheritance + grammar parity

**Status:** Accepted
**Date:** 2026-07-16
**Covers:** The four design gaps surfaced by the 2026-07-16 production-readiness
review of the style system. Locks the CSS shapes Sigil takes, the ones it
rejects, and the contract every utility token must satisfy.

## Context

The readiness review judged the style system against the project goal:
"not a CSS clone, but the general shapes of CSS that transfer across
systems." Verdict: the substrate (slot-based last-wins merge, tokenized
themes, tone x variant x step, state-variant closures) is right, and in places better than CSS. Four shapes were taken halfway:

1. **Inheritance stops at text color.** The `e7f2751` scene-walk cascade
   covers `color` only. Font size/family/weight/line-height reset to
   `TextStyle::default()` on every node (`layout/src/text_style.rs`,
   `renderer/src/style_adapter.rs:91`), so a container's `text-sm
   font-mono` silently does nothing to descendant text. The turn-strip
   WCAG bug was this exact class of failure, for color.
2. **Silent no-op tokens.** `border-*` / `shadow-*` parse into
   `UnknownThemeRef` and never paint. CSS's core contract is "every
   property you write does something"; a token that parses and no-ops is
   worse than a rejected one.
3. **Grammar/atom drift.** Atoms with no tokens (`FlexGrow`,
   `FlexShrink`, `OverflowX/Y`), tokens with no atoms (border/shadow),
   `MinHeight`/`MaxHeight` missing entirely, and
   `style-macros/grammar/structural.toml` (54 entries) out of sync with
   `parser.rs::STRUCTURAL_TOKENS` (67 entries).
4. **Undocumented precedence.** static → dynamic → reactive → state
   overlay exists in code; widgets got it wrong once already
   (turn-strip `class_when` merge bug).

## Decision

### D1. Inherited-property set (CSS shape: inheritance)

Inherited, resolved top-down with own-atom-wins:

- `color` (already shipped, `e7f2751`)
- `font-family`, `font-size`, `font-weight`, `line-height` (this DR)

Everything else (box model, flex, radius, opacity, overflow,
transitions) stays non-inherited, matching CSS.

**Single resolution rule, two consumers.** A pure helper in
`sigil-style` layers a node's typography atoms over an inherited
`TextStyle`. Both walks call it:

- **Layout** (`engine.rs::sync_subtree`) carries the effective
  `TextStyle` down the recursion and seeds text-node measurement from
  it instead of `TextStyle::default()`. Measurement and paint must
  agree or text measures at one size and paints at another.
- **Renderer** (`scene.rs::push_subtree`) extends the existing
  `Inherited { theme, text_color }` carry with the effective typography
  and hands it to glyph-run styling.

**Invalidation.** The ε1 version fast-path in `sync_subtree` skips
re-derivation when a node's own version is unchanged. Under
inheritance a text node can go stale when an *ancestor's* typography
changes. The fast-path therefore additionally compares a hash of the
inherited typography against the value cached at last sync. The
incremental `sync_one_node` path resolves inherited typography by
ancestor walk, and a dirty container whose typography atoms changed
re-derives its descendant text nodes.

Containers without typography atoms pass the inherited value through
by reference (`Cow`); no per-node clone on the hot path.

### D2. No-silent-no-op token contract

Every token accepted by `classes!` or `.class()` must resolve to an
atom with an observable effect. `border-*` and `shadow-*` are
**rejected** at parse time (compile error in `classes!`, `ParseError`
+ diagnostic at runtime) until a stroke/shadow atom path exists. That path is Wave 2 of the production plan (SDF ring primitive;
shadow atoms bridging to the existing `DrawShadow` renderer support).
Rejection now, painting later; never silence.

### D3. Grammar/atom parity

- Add `MinHeight` / `MaxHeight` atoms + `min-h-*` / `max-h-*` tokens
  (symmetry with the existing width pair).
- Add tokens for existing atoms: `grow` / `grow-0` / `shrink` /
  `shrink-0`, `overflow-hidden` / `overflow-auto` / `overflow-scroll` /
  `overflow-visible` (+ `overflow-x-*` / `overflow-y-*`).
- `structural.toml` becomes the single source of truth; a parity test
  asserts (a) the runtime parser accepts exactly the toml token set
  modulo documented dynamic families, (b) every token resolves to a
  non-`UnknownThemeRef` atom except the documented `bg-*`/`text-*`
  theme-passthrough families.

### D4. Documented precedence ladder

The resolution order — theme < variant base < variant state slot <
static classes < dynamic classes < reactive classes < state overlay,
conflicts within a layer last-wins by slot — is documented on
`sigil-style`'s crate root and in the `Styled` trait docs. The ladder
is already enforced by `merge.rs` + binding slice order; this makes it
contract.

## Rejected alternatives

- **Renderer-only typography inheritance** (the color approach):
  unsound: color does not affect layout, typography does, and
  measure/paint would diverge.
- **Resolve inheritance at binding-install time** (store effective
  style as a node extension): pushes ancestor-change invalidation into
  the reactive graph — every node subscribing to every ancestor's
  typography. The walk-carry is strictly simpler and the walks already
  exist.
- **`hover:` / `sm:` string prefixes**: stay rejected (DR-006, DR-049).
  State variants are closures; responsiveness is signals. A future
  viewport-size signal (already a followup) covers container-conditional
  layout the sigil-native way.

## Consequences

- A container's `text-*`/`font-*`/`leading-*` classes now affect
  descendant text, matching every CSS user's expectation.
- **Golden outcome (verified 2026-07-16):** 75 of 105 goldens shifted
  bytes on regen. Isolation against a regen from unchanged base code
  proved 68 were pre-existing environment AA drift (delta ≤1) and were
  restored; exactly 7 changed because of this DR — the five chips,
  status_bar, and pattern_app_shell, all captures of the one widget
  (`chip`) that declares container-level typography. The old bakes
  encoded the pre-inheritance 16px bug; the chip's contrast table
  always assumed 12px. Rebake passed the the reviewer cross-vendor gate
  (AA on all five tones; two pre-existing harness/layout artifacts
  filed in followups).
- `border-*`/`shadow-*` tokens fail loudly at compile time and runtime;
  the grammar had listed them but no in-tree call site used them, so
  nothing migrated.
- The 400-line file caps hold: the layout carry lands in
  `engine.rs`/`text_style.rs`, the renderer carry in `scene.rs` +
  `style_adapter.rs`, the shared helper in `style/src/text_style.rs`.
