# Sigil dogfood — log-tailer FRICTION log

I built a viewer-class app (`log-tailer`) on top of Sigil: ~10k synthesised
log lines through `virtual_list`, a text filter, a level picker, an
auto-scroll toggle, and row selection. Here is what hurt and what didn't.

## Time-to-first-paint estimate

Roughly **35-45 minutes** from "first open `prelude.rs`" to "headless smoke
test green." Most of that was clippy ping-pong, not Sigil itself — Sigil's
public surface is actually quite easy to grok. About 20 minutes of the
total budget went to a single clippy mismatch
(`unreachable_pub` vs `redundant_pub_crate`, see below).

## Discovered by grepping source

These I could not find from the prelude alone:

1. **`virtual_list` is not in the prelude.**
   `crates/sigil/src/prelude.rs:96-102` lists ~20 widget re-exports
   (`button`, `checkbox`, `text_field`, `toggle`, ...) but `virtual_list`
   and `SelectionMode` are missing. They live at
   `crates/sigil/src/widgets.rs:45` and must be imported as
   `use sigil::widgets::{SelectionMode, virtual_list};`. For a viewer-class
   app, virtual_list is THE primitive — surprising that it isn't in the
   prelude alongside `button`.

2. **`derived` lives in `sigil::reactive`, not the prelude.**
   The prelude exports `signal`, `Signal`, `effect`, `Trigger`, `EventChannel`
   (`crates/sigil/src/prelude.rs:76`) but **not** `derived`. I needed it
   for a `Selected row #N` label and only found it after grepping
   `crates/reactive/src/derived.rs`. Found via `sigil::reactive::derived`.

3. **Widget builders (`ButtonBuilder`, `ToggleBuilder`,
   `TextFieldBuilder`, `VirtualListBuilder`) do not implement `LayoutBuilderExt`.**
   They have NO `.layout(...)` method. They have only their own narrow
   self-typed setters (`.width()`, `.height()` on `ButtonBuilder` —
   `crates/sigil/src/widgets/button.rs:111`). To layout a widget you
   wrap it in `container().layout(...).child(widget)`. This is invisible
   from the prelude and there is no rustdoc on the prelude that says
   "widget builders are not NodeBuilders, wrap them in a container."

4. **`text_field` has no way to set width without a wrapper.**
   `ButtonBuilder` has `.width()`, but `TextFieldBuilder`
   (`crates/sigil/src/widgets/text_field/builder.rs`) does not. The
   `vertical-slice` example sidesteps this by giving the text field's
   parent column a fixed width. I had to wrap mine in a sized container.

5. **`VirtualListBuilder::build` requires `IntoNode`-via-`.child(...)`, not
   `.layout(...)`.** I expected the builder to expose layout knobs on
   the viewport itself (it owns the viewport container internally), but
   instead you size the *outer* container and use `viewport_width` /
   `viewport_height` to inform the virtual window. Discovered via
   `crates/sigil/src/widgets/virtual_list/mod.rs:111-125`.

6. **`IntoNode` is the right return type for widget helper fns; `NodeBuilder` is
   not.** When I tried `fn level_button(...) -> NodeBuilder` and returned a
   `ButtonBuilder`, the compiler told me types don't match. The fix is
   `-> impl IntoNode`. The kernel's `child` API is `child(impl IntoNode + 'static)`
   (`crates/kernel/src/builder.rs:164`), but that signature is hidden
   from prelude users.

## Missing widgets / primitives

- **No scroll-to-bottom helper on `virtual_list`.** I had to compute
  `(total_rows * row_height - viewport_height).max(0.0)` myself to seed
  the `scroll_bind` signal at the bottom for auto-scroll. A
  `.scroll_to_bottom_initially()` or `.start_at(Pin::End)` would have
  been obvious. (See `view.rs::initial_scroll`.)
- **No "tail" mode that auto-pins to the end when items are appended.**
  Since this viewer is read-only over a static corpus, I cheated and
  only seeded the bottom once. A real tail would need an
  `auto_pin_to_bottom: bool` field on the builder.
- **No "highlight current selection visually" path I could find on
  `virtual_list`.** `SelectionMode::Single` registers an a11y-level
  selection bit, but it doesn't change visible row styling. To get a
  *visible* selected-row highlight I'd have to wire a per-row
  `style_reactive` that reads from `selection_state()`. Did not finish
  this in budget — selection works behaviourally and reports via the
  status bar, but the row itself doesn't repaint. **Documented as a
  follow-up.**
- **No styled "log level chip" primitive.** I wanted coloured TRACE /
  DEBUG / INFO / WARN / ERROR badges in each row. There is no
  `chip()` / `tag()` / `badge()` widget. I would have hand-rolled this
  with `container().class("bg-warn rounded-sm p-1").child(text(...))`,
  but the style atoms list in the prelude doc-comment
  (`crates/sigil/src/prelude.rs:77-88`) doesn't include `bg-warn`,
  `bg-error`, etc. — only `bg-<name>` where `<name>` is from the theme
  palette. I would need to dig into the theme palette to know what
  semantic colour names exist. For this dogfood pass every row ships
  with the same `text-on-surface` class; **flagged as follow-up**.

## `virtual_list` ergonomics

- The row-builder closure is **`Fn(usize, &T) -> NodeBuilder`** —
  obvious and clean. The `usize` is the absolute index in the source
  vec (not the visible-window position), which is what you want.
- The `Vec<T>` ownership pattern (`items: Vec<T>`) felt right for a
  static dataset; for a "live" tailer I'd want `Signal<Vec<T>>` or a
  borrowed slice. Not present.
- **Confusion**: I initially expected `virtual_list` to *react* to
  scroll changes from inside its own subtree (i.e., scroll the signal
  → it updates the visible window automatically). It does, but only
  on **rebuild** — the visible window is computed at `build()` time
  from `scroll_y.get()`. That means a host needs to invalidate /
  rebuild the subtree when `scroll_y` changes. I'm 80% sure that
  happens automatically through reactive tracking, but I couldn't
  verify it without launching the actual window. **Documented as
  follow-up.**
- The builder's `.selection_state()` method returns a `Signal<SelectionState>`
  but also installs the same signal on the builder for downstream row
  click handlers. The double-meaning (read-and-install) is non-obvious;
  I had to read `mod.rs:159-161` to confirm it.
- The `keyboard(true)` default firing intra-list arrow-key handlers is
  a nice touch — but it's silent. A user dropping in a virtual_list
  doesn't realise their list captures focus on Tab and arrows steal
  Home/End. Worth a doc-comment callout.
- **`scroll_bind` takes a `Signal<f32>`** — not a `&mut f32` or a
  `Cell<f32>`. The reactive coupling is the right choice but I had to
  read the source to confirm I should pass `signal(0.0)`.

## Confusing names

1. **`label` vs `text` vs `TextContent` vs `AccessibleName`.** Four
   different types live close to each other. `text(...)` builds a text
   *node*. `label(...)` is the same thing pre-classed with
   `text-on-surface` (`crates/sigil/src/widgets/shared.rs:17`).
   `TextContent` is the enum the text node holds. `AccessibleName` is
   what the a11y `.label("...")` argument gets coerced into. I picked
   `text` first; should have picked `label` because the prelude exports
   it and the workspace lints want re-export discipline.
2. **`activate` (button) vs `change` (text_field) vs `submit`
   (text_field) vs `pointer_down` (container).** Four different
   "user gestured" hooks all spelled differently. I learned each one
   by example; would have liked a "common widget callbacks" doc page.
3. **`scroll_bind`** — I read it as "bind THE scroll" (singular) but
   it's actually "bind a signal that drives the y-axis scroll." The
   `_y` would make this concrete.
4. **`pub fn fixed_size(w, h)` vs `pub fn width(w).height(h)` on
   `LayoutStyle`.** Two ways to do the same thing; not clear which is
   canonical. The vertical-slice uses both interchangeably.

## Boilerplate I had to copy-paste

1. **`#[allow(missing_docs)]` at the top of `main.rs`.** Every example
   has it; reflects workspace `missing_docs = "warn"`.
2. **The trait-bound dance for `fn row_builder() -> impl IntoNode`.**
   Took me three compile errors to learn that `NodeBuilder` and
   `ButtonBuilder` are unrelated despite both having `.child(...)`.
3. **`Runtime::scope("test-name", || { ... })`** in every test that
   touches a `Signal`. Cargo-cult'd from
   `crates/sigil/tests/virtual_list_integration.rs:42`.
4. **The `#![allow(clippy::redundant_pub_crate)]` crate-root attribute.**
   Without it, a binary with submodules deadlocks between two clippy
   lints: `unreachable_pub` says "use `pub(crate)`", but
   `redundant_pub_crate` says "private module, use `pub`." Neither lint
   knows about the other. I would have welcomed a `clippy.toml` note
   saying "binaries with internal modules: allow one or the other."

## Things that surprised me (good)

1. **`Signal<T>` is `Copy`.** This is the right call — it means
   `AppState { ... fields: Signal<_> ... }` can also be `Copy` and
   every closure capture is cheap. Made the `pass-by-value` clippy
   warnings disappear once I added `#[derive(Clone, Copy)]` to
   `AppState`.
2. **`sigil::launch(("title", w, h), build_fn)`** is genuinely a
   one-liner. Took me 30 seconds from "open hello-counter" to
   understand the launch shape.
3. **The headless test surface is **excellent**.** `Cx::new_root()`
   plus `Runtime::scope(...)` lets me assert tree-builds-without-panic
   in 8 lines. No window, no GPU, no event loop. This is the difference
   between "I shipped" and "I shipped and slept well."
4. **Class strings (`"p-4 bg-surface rounded-md"`)** parse at build time
   and the list of supported atoms is documented inline in the prelude
   (`crates/sigil/src/prelude.rs:77-88`). Tailwind-ish ergonomics
   without runtime cost.
5. **The reactive `Signal<String>` + `text(signal)` coupling is
   transparent.** No subscribe boilerplate, no wiring — `text(my_signal)`
   just works.

## What I'd change if I owned this API

1. **Re-export `virtual_list`, `SelectionMode`, `SelectionState`, and
   `derived` from `sigil::prelude`.** These four are the difference
   between "I can build a viewer" and "I have to grep the source."
2. **Make widget builders implement `LayoutBuilderExt`** so
   `.layout()`, `.width()`, `.height()` work uniformly across
   `ButtonBuilder`, `ToggleBuilder`, `TextFieldBuilder`,
   `VirtualListBuilder`, `NodeBuilder`. Today each builder reinvents a
   subset; some skip it entirely.
3. **Add `.scroll_to_end()` and `.auto_pin_to_bottom(bool)` on
   `VirtualListBuilder`.** The "tail -f" use case is too important to
   force every user to compute it.
4. **Resolve the `unreachable_pub` vs `redundant_pub_crate` clippy
   deadlock for binary crates with submodules** — either by
   suppressing one of the lints at workspace level for `[[bin]]`
   targets, or by documenting the recommended escape hatch in
   `examples/README.md`.
5. **Add a `chip(label, tone)` primitive** for the
   level-badge / status-pill use case. It's a 30-line widget that
   would save every viewer / log-tail / status-list app from
   rebuilding the same `container().class("rounded-sm p-1 bg-warn")`
   pattern.

## Follow-ups (out of scope for this dogfood)

- Selection visual highlight on the row (currently a11y-only).
- Per-level row colouring (chip widget would help).
- Verify that `scroll_y` reactivity rebuilds the visible window
  automatically when scrolled in a real window (headless test only
  confirms initial paint).
- Promote `virtual_list` / `SelectionMode` / `SelectionState` /
  `derived` into the prelude.
- Live tail mode (append to `Vec<LogLine>` and rebuild). Currently
  the corpus is static and `Box::leak`'d in `AppState::new`.
- Full-app pixel golden — the brief allowed best-effort. Did not
  attempt; the `vertical-slice` dev-deps pattern would have added 20+
  minutes to set up the test render harness, and I prioritised an
  honest friction log over a golden screenshot.
