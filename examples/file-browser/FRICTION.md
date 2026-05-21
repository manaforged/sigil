# Dogfood FRICTION log — `examples/file-browser`

Multi-pane mock file browser (Tier 4). Composes `splitter` + `virtual_list` + `tabs` with content routing. Selection in the left pane re-routes the active tab body on the right.

---

## Time-to-first-paint estimate

About **35 minutes from `mkdir examples/file-browser` to `cargo test ... ok`**. ~20 min reading sigil source / tests, ~10 min writing four files, ~15 min grinding the clippy/lint hook loop. The clippy loop alone burned more time than writing the actual app.

---

## Discovered by grepping source (not findable from the prelude)

These public items are not re-exported by `sigil::prelude::*` even though they're the central widgets for a multi-pane app. Every one had to be found by reading `crates/sigil/src/widgets.rs` directly.

- `sigil::widgets::splitter`, `SplitterOrientation`, `SplitterBuilder` — `crates/sigil/src/widgets.rs:40`. Re-exported from `widgets` but **NOT** from `prelude`.
- `sigil::widgets::virtual_list`, `SelectionMode`, `SelectionState`, `VirtualListBuilder` — `crates/sigil/src/widgets.rs:45`. **NOT** in prelude.
- `sigil::widgets::tabs`, `TabsBuilder` — `crates/sigil/src/widgets.rs:41`. **NOT** in prelude. (Confusingly, `tabs` IS listed in `sigil::prelude` per `crates/sigil/src/lib.rs:97`, but I had to re-confirm by grep because the prelude lists the imports inline as `tabs, TabsBuilder` rather than grouping them by widget.)
- `sigil::reactive::derived` — needed for the `Signal<String>` body of each tab. The prelude pulls `effect` and `signal` (`crates/sigil/src/lib.rs:76`) but **not** `derived`. Had to import via `use sigil::reactive::derived;` separately.
- `VirtualListBuilder::selection_state(&mut self) -> Signal<SelectionState>` — `crates/sigil/src/widgets/virtual_list/mod.rs:159`. The mechanism for wiring selection into host state. Had to read the source — it's not in `prelude`-level docs.
- `Signal<SelectionState>::get().selected_indices()` — `crates/sigil/src/widgets/virtual_list/selection.rs:62`. The iterator API for reading current selection. Found by reading `selection.rs`.
- `IntoNode + use<>` precise capturing — needed in Rust 2024 to return a non-borrowing `impl IntoNode` from a function that takes `&[T]`. Discovered by rustc error message, not docs. Without it, the compiler infers that the slice lifetime is captured by the return type and `static` constraint on `SplitterBuilder::child` fails.

---

## Splitter + tabs + virtual_list composition ergonomics

**Did the splitter children clamp correctly?** Mostly. I had to hand-compute the pane widths (`360.0` left, `656.0` right) to add up to the window width (`1024.0 − 8.0` for the two 6-px handle). There is no `splitter().fill()` or auto-stretch — every pane needs a literal pixel size at build time. `min_size` works as a hard floor.

**Did `virtual_list` inside a splitter pane size correctly?** I had to set `viewport_width(360.0)` and `viewport_height(540.0)` on the `virtual_list` manually. Without explicit `viewport_width`, the doc says it uses `100%` but in practice I never felt confident the layout was actually consulting the splitter's pane width — easier to hard-code than reason about. This is the worst smell: two widgets that *should* compose with intrinsic sizing have to be reconciled by hand-computed pixel constants.

**Did clicking a tab re-route content cleanly?** Yes — `TabsBuilder::content_for(idx, node)` works exactly as the docs claim. Inactive panels collapse to `Display::None` and the active one paints. This is the one part of the composition story that felt finished.

**Layout fights?** One real one: my first `right_pane::build` took `&[MockFile]` and returned `impl IntoNode`, but Rust 2024's impl-Trait lifetime capture made the returned future-trait-object hold the slice's lifetime, which fails the `'static` bound on `SplitterBuilder::child`. The fix was either `+ use<>` (precise capturing) or taking `Vec<MockFile>` by value. The error message from rustc was clear, but if you're new to Rust 2024 this is a sharp edge.

---

## Selection state propagation

**Idiomatic?** Mostly, but with a non-obvious dance:

1. Build the `virtual_list` builder.
2. Call `.selection(SelectionMode::Single)`.
3. Before calling `.build()`, call `builder.selection_state()` to get back the `Signal<SelectionState>` it will own.
4. Wire an `effect` that reads `selection.get().selected_indices().next()` and writes to your host `Signal<usize>`.
5. The right pane consumes the host signal via `derived(...)` for each tab body.

The friction: `selection_state()` is `&mut self`, not `self`, so the builder is consumed *afterwards* by `.build()` (or implicitly when passed to `splitter().child(...)`). I had to look at the integration test (`virtual_list_integration.rs:115`) to see the pattern.

**Why not idiomatic:** there's no `.on_select(|idx| ...)` shorthand. Every consumer has to set up the `effect` mirror manually. That's three or four lines of boilerplate per virtual-list.

---

## Missing widgets / primitives

- **No `on_select` callback on `VirtualListBuilder`.** The selection-state Signal is the API. Hooks need `effect` + the iterator-into-first-selected dance.
- **No `divider` / `separator` widget.** I wanted a thin horizontal line between the title and the splitter; ended up not bothering.
- **No `header` / `panel-with-title` widget.** Hand-rolled a flex_column with a text header.
- **No theme tokens for monospace text in atoms.** I used `.class("font-mono text-sm")` — works because the utility-class parser accepts it (`crates/sigil/src/lib.rs:79-88`), but I never confirmed the renderer actually swaps to a monospace face. There's no headless way to assert "the hex tab is monospaced."
- **No `scroll_into_view` on virtual_list.** If selection moves to a row outside the viewport (e.g. set programmatically), the user won't see it.

---

## Confusing names

- **`signal` vs `Signal` vs `signal()`** — three things named the same thing. `sigil::reactive::signal(value)` constructs; `sigil::prelude::Signal<T>` is the type; the convention is just "you'll figure it out." It works but I had to think about it.
- **`SplitterBuilder::child(node, size)` takes the size as the second positional arg.** Every other builder in the crate uses `.layout(LayoutStyle::...)` or fluent methods for sizing. Having a positional `f32` makes the call site read like a tuple. `.pane(node).initial_size(200.0)` would be self-documenting.
- **`TabsBuilder::content_for(index, node)`** vs `SplitterBuilder::child(node, size)` — both attach content but use different argument orders and different names. Pick one.
- **`Role::ListItem`** — used inside `virtual_list` rows. Fine, but `Role::GenericContainer` is the catch-all for left/right pane wrappers; the name is awkward (it's accesskit's name; the prelude inherits it).

---

## Boilerplate I had to copy-paste

- The `effect(move || { if let Some(first) = selection.get().selected_indices().next() { selected_index.set(first); } })` block. Three lines of "mirror selection state into a usize signal" per virtual_list.
- Per-tab `derived(move || { let idx = selected_index.get().min(count.saturating_sub(1)); files.get(idx).map(...).unwrap_or_default() })`. Each of the three tabs needs the same `idx-clamp + get + map + unwrap_or_default` shape. A `derived_indexed(signal, files, |file| ...)` helper would have collapsed three near-identical functions to one.
- The `+ use<>` annotation on every `fn build(...) -> impl IntoNode + use<>` returning into a `+ 'static` slot. If you don't put it there, the compiler grabs lifetime captures you didn't authorise.

---

## Things that surprised me (good)

- **Tabs content routing JustWorks.** I passed three nodes to `content_for(0/1/2, ...)`, set `active` to a signal, and the right widget collapsed inactive panels via `Display::None` without me thinking about it. This is the cleanest piece of API in the example.
- **`signal(0_usize)` + `derived(move || ...)` for tab bodies is genuinely reactive.** Clicking a row in the left pane updates `selected_index`, which propagates through three `derived` signals (one per tab), and each tab's body re-renders. Zero kernel-tree rebuilds. The architecture works.
- **The smoke test (`cargo test`) runs a full `Cx::new_root() + app_tree(&mut cx)` build under `Runtime::scope(...)` and finishes in under 1 ms.** A 1-line test gives me confidence that the entire widget tree composes without panic.
- **`sigil::launch((title, w, h), app_tree)` accepts a tuple as config.** Felt natural. Discovered by reading hello-counter.

---

## What I'd change if I owned this API

1. **Re-export the big-three widgets in the prelude.** `splitter`, `SplitterOrientation`, `virtual_list`, `SelectionMode`, `SelectionState`, `derived`. Anyone building a multi-pane app needs them all; making them sub-module imports just adds search friction.
2. **Add `VirtualListBuilder::on_select(impl Fn(usize))`.** Shorthand for the effect-mirror dance. Internally, build the effect.
3. **Rename `SplitterBuilder::child(node, size)` → `.pane(node).size(px)`** (or take a `(impl IntoNode, f32)` tuple and name it `panes(&[(node, size), ...])`). Positional `f32` after a node arg is bait for ordering bugs.
4. **Document the Rust-2024 `+ use<>` requirement** prominently in the splitter/tabs/virtual_list rustdoc. The rustc error is clear but the docs imply "just return `impl IntoNode`."
5. **Give `LayoutStyle` an `auto_size_from_parent()` (or similar)** so I can put a `virtual_list` in a splitter pane without manually copying the pane width into `.viewport_width(...)`. Right now layout sizing is bottom-up: every leaf has to know its own pixel dimensions, which makes responsive composition impossible.

---

## Lint-hook friction (separate from API)

The PostToolUse `rust-postedit.mjs` hook fired clippy after every Edit/Write. Each round surfaced a different lint:

- `unreachable_pub` (fires on `pub fn` in private modules) and `redundant_pub_crate` (fires on `pub(crate) fn` in private modules) **directly contradict each other**. The only path through is `pub mod foo;` at the crate root for any sub-module that exposes anything sibling-visible. This took 3-4 lint-loops to resolve.
- `doc_markdown` flagged `virtual_list` and `selected_index` in my module docs because they look like Rust identifiers but weren't backtick-wrapped. Fair enough but loud.
- `format_push_string` fired on every `out.push_str(&format!(...))` — wanted `let _ = write!(out, ...)`. Cleaner code in the end, but I wouldn't have reached for the lint myself.
- `cast_possible_truncation` on a deliberate truncation (`(mixed >> 16) as u8`). Required `#[allow(clippy::cast_possible_truncation)]` with a comment.
- `too_many_lines` fires at 50 lines per fn. My `inventory()` literal was 86 lines of static data. Had to split into `home_entries()` + `system_entries()` to stay under cap. Reasonable rule, mild churn.

None of these are sigil API problems — they're the author hygiene rules. Noting them because a new sigil user without that hook chain would see only the doc-markdown one and call it a day.
