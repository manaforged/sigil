# api-explorer — friction log

Dogfood pass v2, tier 4 (splitter + virtual_list + tabs).
Built 2026-05-20. ~450 LoC across 4 files.

## Time-to-first-paint

About 35 minutes from `mkdir` to green `cargo test`. The hello-counter
and file-browser exemplars made the wiring path obvious; most of the
time went into the mock catalog content (had to keep the JSON-ish bodies
plausible) and one round-trip with the hook chain.

Rough breakdown:

- 4 min — read the prelude, the file-browser exemplar's three files,
  and the splitter / tabs / virtual_list public surfaces.
- 18 min — write `endpoints.rs`, `left_pane.rs`, `right_pane.rs`,
  `main.rs`, `Cargo.toml`.
- 6 min — hook back-and-forth: workspace `members[]` append, leaf-file
  module ordering (see "boilerplate" below), two clippy fixes.
- 7 min — full verification gate (cargo check + build + clippy + test +
  fmt --check).

## Discovered by grepping source

- `crates/sigil/src/prelude.rs:98` — `splitter`, `tabs`, `virtual_list`,
  `SelectionMode`, `SplitterOrientation` are all re-exported. No need
  to reach for `sigil::widgets::*`, which matched the brief's claim
  that the prelude was expanded.
- `crates/sigil/src/widgets/virtual_list/mod.rs:195` — `.select(f)` is
  the new shortcut. The docstring is explicit about the
  "fires-once-on-build-if-already-selected" behaviour, which is the
  exact thing a host needs to know to avoid double-fire bugs.
- `crates/sigil/src/widgets/tabs/builder.rs:174` — `.content_for(idx,
  node)` is the routing hook. Nothing in the prelude advertises it;
  found it by reading `examples/file-browser/src/right_pane.rs:43`
  and grepping the builder.
- `examples/file-browser/src/right_pane.rs:51` — the `derived(move ||
  { let idx = selected_index.get().min(count.saturating_sub(1)); ...
  })` pattern is the canonical way to route a per-selection projection
  into a tab body. Copy-paste-with-rename worked.

## splitter + tabs + virtual_list composition ergonomics

This is the central question for the tier. Findings:

### Splitter clamping

Worked as advertised. `splitter(SplitterOrientation::Horizontal)
  .child(left, 360.0)
  .child(right, 656.0)
  .min_size(0, 200.0)
  .min_size(1, 320.0)` reads naturally; no measurable surprise. The
`min_size(idx, px)` API takes a positional child index, which is fine
for two-pane layouts but I notice a `min_size_first` / `min_size_last`
named shortcut would scale better at three+ panes.

### Virtual list sizing inside a splitter pane

Still required hand-computed pixel sizes. I set the list's
`viewport_width(360.0)`, `viewport_height(540.0)`, and the pane's
`fixed_size(360.0, 600.0)` separately. The constants have to agree
or the row hit-test misalignments show through. The previous dogfood
pass reportedly hit this; it's still the case. See the API change
proposals below for `viewport_fill()`.

### Tab routing

`.content_for(0, panel) .content_for(1, panel) .content_for(2, panel)`
worked cleanly. The tab labels are owned-`String` (because the brief
calls them dynamic) but the content nodes are `impl IntoNode +
'static`, which is the right boundary — no allocation churn per click,
and `derived` signals route the actual body re-render.

The "build all three panels once at app_tree build" pattern from
file-browser was load-bearing: trying to build the panels inside a
match arm in `content_for` would have required N closure-builders.
Worth surfacing as a doc snippet in the tabs builder.

## Selection state propagation

`.select(move |index| selected_index.set(index))` worked exactly as
the docstring claims. One round-trip: I expected the closure to fire
on click only, but per the docstring it also fires once synchronously
during build because the initial selection (`signal(0_usize)`) is
already in `SelectionState`. That was fine — my right-pane derived
signals are designed to handle it — but the docstring's "Initial
sync-fire" paragraph is the load-bearing bit; without it I'd have
written a `first_call` guard.

## Missing widgets / primitives

- **No `kv_grid` or similar.** Headers ("status: 200 / latency: 38 ms")
  are rendered as a manually-formatted multi-line string inside a
  `font-mono` text block. A 2-column key-value grid widget would be
  the right answer; I made do.
- **No syntax-highlighted code block.** Request / Response bodies are
  JSON; rendering them as `font-mono text-sm` is fine for a dogfood
  example but a real app would want a `code_block(lang: "json", body)`
  primitive. Not blocking.
- **Selection-derived signal is DIY.** The pattern
  `selected_index.get().min(count.saturating_sub(1))` is repeated 3×
  across the three tab bodies. I extracted it into
  `endpoint_derived(endpoints, selected_index, |e| ...)` locally,
  which is the right shape for a future `VirtualListBuilder` method
  like `.project::<T>(|item| ...) -> Signal<T>`.

## Confusing names

- `Method` lives in the example crate as a `&'static str` newtype.
  Wanted to call it `HttpMethod`. Kept it short because it's
  example-local.
- `splitter().child(node, size)` — the second arg is the initial
  size, not a size cap. Read as `child(node, initial_px)` if you
  haven't seen it before. A named `.pane(node).initial(size)` builder
  would be clearer, but probably not worth the API churn.
- `LayoutStyle::flex_column().fixed_size(W, H).gap(N)` — the
  `fixed_size` call is the heaviest boilerplate of the whole file
  (appears 6× across the 4 files). The brief mentions a `viewport_fill()`
  consideration in proposals.

## Boilerplate copy-paste

- **Per-file module ordering with the postedit hook.** The hook runs
  `cargo fmt -p <crate>` after every Write, and `cargo fmt` resolves
  every `mod foo;` declaration eagerly. So writing `main.rs` first
  (which has `pub mod endpoints; pub mod left_pane; pub mod
  right_pane;`) blocked the format pass because `left_pane.rs` /
  `right_pane.rs` didn't exist yet. Workflow has to be:
  leaf modules first → main last, OR stub-out main with empty
  `mod foo {}` blocks first. Not the example's fault; surfacing as
  hook-ergonomics friction.
- **`LayoutStyle::flex_column().fixed_size(...).gap(...)`** appears at
  every container. A pre-baked `LayoutStyle::pane(W, H)` shortcut
  (column + fixed + gap-4) would cut ~6 lines.
- **`text(...).layout(LayoutStyle::block().width(W)).class("...")`**
  appears 5×. An `.constrained_text(s, width)` helper or just allowing
  `text(s).width(W)` directly would be nice.
- **Three near-identical `*_tab` functions in right_pane.rs.** They all
  do `endpoint_derived(...) → panel_body(text(body).font-mono)`. I
  collapsed the derived part into one helper; the final pattern is
  acceptable for the example tier but a `tab_body(label, |e| String)`
  combinator would be cleaner.

## Surprised-good

- **`.class("font-mono text-sm")` worked first try.** The Tailwind-ish
  utility-class strings from the prelude doc-comment are real; no
  surprise about which atoms exist. The class-string discoverability
  via the prelude `mod prelude { ... pub use sigil_style::... }` doc
  comment is genuinely useful.
- **`SelectionMode::Single` + `.select(...)` is two lines.** Compared
  to the imagined "wire a selection-state signal, observe it, then
  reduce to first-index in an effect," the `.select` shortcut elides
  all of it.
- **`derived(move || ...)` returns a `Signal<T>` that's directly
  acceptable as `text(body)`.** No `.subscribe()` ceremony. The reactive
  primitives compose with the widget builders without an adapter type.
- **The `splitter` widget defaults work.** I didn't need to set any
  drag handle styles, hit-test widths, etc. Just `.child(a, w1)
  .child(b, w2) .min_size(...)` and the splitter renders.

## API change proposals

1. **`VirtualListBuilder::viewport_fill()`** — read the parent
   splitter pane's measured size at layout time and skip the
   `viewport_width(360.0).viewport_height(540.0)` boilerplate.
   Hand-computed pixel sizes inside a splitter pane is the single
   biggest hot-spot for "I edited the wrong constant" bugs and is
   exactly what the brief flagged from the previous pass.

2. **`SplitterBuilder::pane(node).min(px).initial(px)`** — replace
   the positional `child(node, initial)` + `.min_size(idx, px)` chain
   with a per-pane builder. Currently the index-based `.min_size(0,
   200.0)` is the only thing in the chain that uses a positional
   identifier, and it's brittle when you reorder panes.

3. **`tabs(active, labels).panel(idx, body)`** as an alias for
   `.content_for(idx, body)` — `panel` is the noun authors say out
   loud; `content_for` is a verbose preposition. Keep `content_for`
   for back-compat.

4. **`derived_index_clamped(signal, len) -> Signal<usize>`** in the
   reactive prelude — the `.get().min(count.saturating_sub(1))`
   pattern shows up in every selection-routed surface (file-browser,
   api-explorer, kanban presumably). One named helper would lock down
   the saturating-sub edge case and remove ~4 lines per tab body.

5. **`LayoutStyle::pane(w, h)`** as a sugar for
   `flex_column().fixed_size(w, h).gap(4.0)` — the most-repeated
   layout shape across the multi-pane examples.
