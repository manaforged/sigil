---
id: DR-057
status: accepted
date: 2026-07-16
covers: Best-in-class data tier — headless table core + tone-native chart layer, distilled from the 2026 field survey
depends_on: [DR-031, DR-032, DR-049, DR-056]
relates: docs/plans/2026-07-16-production-readiness.md
---

# DR-057 — Data tier: headless table + tone-native charts

**Status:** Accepted
**Date:** 2026-07-16
**Covers:** The design lock for Sigil's data-heavy flagship widgets. The
2026-07-16 competitive survey found the data table is the single
most-cited reason adopters abandon native Rust UI for Electron, and
charts plus virtualized tables are the two features gpui-component
holds over every other catalog. Sigil's thesis is data-heavy desktop apps;
this tier is the thesis made visible.

## Principle: distill shapes, not code

Each design below names the shape it distills and where it came from.
No API is copied; the shapes are re-derived on Sigil's substrate
(signals, tone scales, virtual_list, the R2 stroke primitive, slug).

## D1. Headless table core

**Distilled shape:** logic/presentation separation (the pattern that
made TanStack Table the web's default: the "table" is a state engine
and rendering belongs to the consumer). Rejected alternative: a
monolithic styled-table widget (the AG-Grid/gpui-component shape),
powerful but
couples every feature to one look and balloons the widget file past
every cap this repo has.

- `TableState<Row>`: column definitions (id, accessor, width policy,
  comparator, filter predicate), and sort/filter/selection/visibility
  each held as signals. Pure logic, no node tree, no renderer types:
  unit-testable headless, living beside the widget as
  `widgets/table/state.rs` + `columns.rs`.
- `table(state)` builder renders through `virtual_list` (rows are the
  existing virtualized path; 100k-row scroll inherits its budget) with
  a sticky header strip. Cells are ordinary NodeBuilders — any widget
  can live in a cell.
- Row striping, hover, selection tones resolve through the theme's
  gray/accent scales with the same guaranteed-contrast machinery as
  variant_kit. Grid lines use the R2 `Stroke` primitive once DR-056
  Wave 2 wires border atoms.
- A11y from day one: `Role::Table/Row/Cell` semantics with
  sorted-state announcements through the existing AccessKit bridge.
  This is the field's weakest axis and Sigil's standing advantage.
- Inline editing arrives with the Wave-4 text_edit caret work; the
  column API reserves an `editor` slot so the shape does not break.

**Acceptance:** headless state tests (sort/filter/selection over 100k
synthetic rows, budgeted), composed-scene golden of a styled table,
keyboard navigation test (arrow/home/end/page), a11y tree assertion,
and a scroll bench beside virtual_list's existing budget.

## D2. Tone-native chart layer

**Distilled shape:** grammar-lite mark composition (Observable
Plot/vega-lite's marks × scales × guides, minus the JSON DSL; Rust
builders instead), and shadcn/ui's insight that charts adopting the
host design system beat chart libraries that bring their own look.
Rejected alternative: wrapping plotters/plotly, which means foreign
styling, no tone integration, and no reactive updates.

- `chart(data)` builder with marks: `bar`, `line`, `area`, `point`;
  scales (linear/band/time), axes + gridlines as guides.
- **Color is the differentiator:** categorical series colors derive
  from the theme's tone scales; sequential/diverging ramps derive from
  scale steps, so every series/label combination inherits the AA
  guarantees the tone system already enforces. No competitor's chart
  layer has design-system-native color with measured contrast.
- Rendering v1 uses what ships today: bars/axes/gridlines/ticks are
  rects + R2 strokes; line and area marks rasterize their polyline
  through the slug band pipeline (the icon baker's path already
  converts curves → bands → alpha mask) onto the poly-sprite path,
  tinted by theme color at draw time. v2 (only if animation-heavy
  charts demand it): GPU polyline stroking, which queues the
  shader-check gate.
- Reactivity: data enters as signals; a changed series re-bakes only
  its own mark's mask (mark-level invalidation, mirroring the DR-056
  signature-pruning shape).
- Legends, tooltips (existing tooltip widget), and hover-nearest
  hit-testing ride the existing events router.

**Acceptance:** goldens for each mark type in light/dark, a measured
contrast assertion on generated categorical palettes (extend the
theme_tests contract), a 10k-point line re-bake budget, and the
cross-vendor visual gate on the composed dashboard scene.

## D3. Stale-ceiling correction

The radio-mark / text_field ceiling comments and the Wave-2 plan text
say the renderer "has no stroke/annulus primitive." That is stale:
`DrawRect::with_stroke` + `Stroke { width, style, align }` shipped
with R2 (`scene_command/fills.rs`). Wave 2's border work is therefore
**wiring**, not a new shader: Border atoms (width/color/tone-step) →
RenderStyle → `push_node` → `with_stroke`. The `border-*` tokens
rejected by DR-056 D2 flip to painting at that moment. Chart axes and
table grid lines consume the same wiring.

## Sequencing

Data tier becomes Wave 3 of the production plan, immediately after
Wave 2 (whose composed-scene goldens and border wiring are its
prerequisites) and ahead of packaging, because the flagship widgets
are what a 0.1 announcement is judged on. Packaging shifts to Wave 4, capability
completion to Wave 5.

## What we deliberately do not build

- No pivot tables, row grouping, or tree-grid in v1 (reserved column
  API slots, no machinery).
- No 3D charts, no animation timeline system (the existing animation
  runtime covers mark transitions when needed).
- No chart DSL strings — builders only, same as the rest of Sigil.
