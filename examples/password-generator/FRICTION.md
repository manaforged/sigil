# password-generator — dogfood friction log

Dogfood-pass-v2 (post-gap-closure). Tier-1 utility: slider for length,
four checkboxes for character classes, a button to reroll, and a
derived password readout. ~190 lines of `main.rs`.

## Time-to-first-paint

- **Honestly: ~25 minutes from "open repo" to green gate.** That breaks
  down as ~5 min reading `hello-counter` + `timer-stopwatch` for the
  shape of `sigil::launch`, ~5 min skimming `prelude.rs` and `widgets.rs`
  to confirm `slider` / `checkbox` / `button` exist and find their
  signatures, ~5 min writing the file, ~10 min clippy-grinding through
  pedantic lints inherited from the workspace.

- The prelude is **massively** improved since I last looked — `signal`,
  `derived`, `slider`, `checkbox`, `button`, `container`, `text`,
  `LayoutStyle`, `Cx`, `NodeId`, `NodeBuilder` are all one `use
  sigil::prelude::*;` away. No more chasing sub-crate names.

## Discovered by grepping source (not docs or prelude)

These are the things I had to read source for, because they were
either undocumented in the prelude or behave differently than the name
implies:

- **`Signal<T>` is `Copy`** —
  `crates/reactive/src/signal.rs:35` `impl<T: 'static> Copy for Signal<T> {}`.
  This is **load-bearing** for ergonomic closures: I can capture a
  `Signal<bool>` by value into multiple `move ||` closures and they
  all see the same reactive cell. I assumed I'd need `.clone()` or
  `Rc` wrapping. Worth surfacing in the prelude doc-comment.

- **`Signal::peek()` vs `Signal::get()`** — not in the prelude doc.
  Had to read `timer-stopwatch/src/main.rs:83` to learn that handler
  bodies use `.peek()` (no dependency tracking) and `derived(||...)`
  bodies use `.get()` (tracks). If you use `.get()` in a button
  handler you'd create a permanent subscription off the click, which
  is a footgun. Surface this in the `Signal` rustdoc.

- **`sigil::testing::with_cx`** —
  `crates/sigil/src/testing.rs:22`. Not in the prelude (it lives at
  `sigil::testing`), but the timer-stopwatch test imports it directly.
  Worth advertising in the launch-page rustdoc since every smoke test
  needs it.

- **`Cx::visit_preorder` / `Cx::first_child`** — found via grep on
  the timer-stopwatch test. No rustdoc on prelude for these. Smoke
  tests can't actually assert "tree has N nodes" without them.

- **`slider(value, range)` returns `SliderBuilder`, not `NodeBuilder`** —
  `crates/sigil/src/widgets/slider/builder.rs:91`. Building it via
  `.child(slider(...))` works because `SliderBuilder: IntoNode`, but
  if you try to compose it like a `container()` you'll get confusing
  type errors. The `IntoNode` impl is invisible from the prelude.

- **`.activate(impl Fn() + 'static)` on `button`** —
  `crates/sigil/src/widgets/button.rs:121`. I expected `.on_click` or
  `.click` (DOM-influenced). `activate` is correct (it's both pointer
  AND keyboard activation) but unguessable. The naming convention
  isn't documented anywhere visible.

- **`.change(impl Fn(f32))` on `slider`** —
  `crates/sigil/src/widgets/slider/builder.rs:129`. I never needed it
  because I'm writing back to a `Signal<f32>` directly, but I went
  looking for it. The pattern "change handlers vs reactive sink" needs
  a one-paragraph explanation in the slider rustdoc — otherwise you
  end up writing both, double-firing.

## Missing widgets / primitives

- **No clipboard primitive** that I could find for the password
  readout. There's `widgets::Clipboard` re-exported from `text_field`,
  which I suspect is a text-field-local clipboard surface — not a
  "copy-to-system-clipboard" action. A `clipboard().set(text)` API
  would be the obvious tier-1 utility primitive. (I dropped the "Copy"
  button from my original sketch for this reason.)

- **No `.disabled_when(Signal<bool>)` on `button`** — only
  `.enabled(bool)` taking a literal. So if you want "Generate disabled
  when no class is selected" you can't bind it reactively without
  rebuilding the tree. (Slider + checkbox + toggle all have
  `.disabled(Signal<bool>)`. Button doesn't. Inconsistent.)

- **No number-input / spinbox.** Length is a slider here because
  that's the closest match. A typed `number_field(Signal<i32>,
  range)` would be a clean tier-1 primitive — sliders are great for
  approximate values, terrible for "I want exactly 24."

- **No live-redraw for `text(Signal<String>)`** — actually this DOES
  work; the derived `password` signal updates the readout on every
  input change. Good. Putting this in the "surprised me (good)"
  section.

- **No icons in the button** — `button("Generate")` takes only text.
  For "copy to clipboard" you really want a glyph. There's a
  `sigil-icons` crate but I didn't dig into how to wire it into a
  button.

## Confusing names

- **`slider(Signal<f32>, RangeInclusive<f32>)`** is f32-typed. There's
  no `slider_int` or `slider::<i32>`. Every integer-style slider has
  to round + cast on read. Documented in my main.rs comment, but the
  ergonomic answer is a typed `slider::<T>` or an explicit `int_slider`.

- **`checkbox` vs `toggle`** — both take `Signal<bool>`, both have
  `.label()`, `.change()`, `.disabled()`. The only obvious difference
  is visual (checkbox is a square indicator, toggle is a switch). I
  picked `checkbox` because that's what password-generator UIs use.
  A one-line rustdoc on each saying "this is for X usage, this is
  for Y" would settle it.

- **`use sigil::reactive::derived;`** — `derived` IS in the prelude
  (line 77), but it's *also* exported under `sigil::reactive` and
  `hello-counter` imports it from there. Pick one path and stick to
  it in the examples.

- **`container()` vs `panel()`** — both produce a node with children.
  `panel()` is presumably "styled container with surface bg" but the
  prelude doesn't say. I used `container().class("bg-surface")` and
  it worked but I'm not sure if I should have used `panel()` instead.

## Boilerplate I had to copy-paste

- The smoke-test scaffold:
  ```rust
  with_cx("name", |cx| {
      let root = app_tree(cx);
      let mut count = 0_usize;
      cx.visit_preorder(root, |_, _| count += 1);
      assert!(count >= N, ...);
  });
  ```
  Verbatim from `timer-stopwatch`. Could be `sigil::testing::tree_node_count(app_tree)` returning a usize, removing the visitor boilerplate.

- The `#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]` on the test module. Workspace pedantic lints are heavy; every example will write this. Worth a `sigil::testing::test_attrs!` macro or a documented snippet.

- **`#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]`** at the file root. Any app that reads f32 sliders into integer logic eats this. The workspace lints inherit from a library-grade pedantic profile. Examples might want their own lint table with these two relaxed (alongside `unwrap_used`/`panic`/`indexing_slicing` in tests).

- The fixed-size container ceremony:
  ```
  .layout(LayoutStyle::flex_column().fixed_size(W, H).padding(P).gap(G))
  .class("bg-surface")
  ```
  Every example writes this. A `screen(w, h).padding(p).gap(g)` helper would cut 4 lines per app. (`sigil_app::launch` already knows the window size — there's a missing "root container" wrapper that the launch fn could inject.)

## Things that surprised me (good)

- **`text(Signal<String>)` just works.** I pass a derived signal of
  the password and it re-renders on every input change. No `Memo`, no
  `Subscriber`, no `cx.spawn_effect`. Beautiful.

- **`signal(initial_value)` is the entire API for reactive cells.**
  No `RwSignal`, no `ReadSignal`/`WriteSignal` split, no `WithUntracked`
  ceremony. One word. Same primitive for `i32`, `bool`, `String`,
  `Vec<u64>`, `Option<Instant>`. This is the cleanest reactive surface
  I've used in Rust.

- **`button("Generate").primary().activate(|| ...)`** — the chained
  builder reads like English. Tone selectors as bare methods
  (`.primary()`, `.neutral()`) are exactly the right level of typed.

- **Workspace cargo deps mean my `Cargo.toml` is 14 lines.** Just
  `sigil = { workspace = true }`. No version bumps to chase.

- **`derived(move || ...)` captures `Copy` signals by value** and the
  closure auto-tracks dependencies. I never had to explicitly declare
  dependencies — they're inferred from `.get()` calls inside.

- **`signal::<f32>(16.0)` flows through `slider(value, 4.0..=64.0)` and
  the slider writes back to the signal automatically.** No event
  callback needed; the signal IS the source of truth. This is exactly
  the right reactive shape for form-style apps.

## What I'd change if I owned this API

1. **Add `sigil::clipboard()` as a tier-1 primitive.** Every utility
   app (color picker, password gen, JSON pretty-printer, hash
   calculator, ...) needs "copy this string to system clipboard." A
   missing primitive forces every dogfooder to either skip the
   feature or pull in `arboard` themselves.

2. **Add `.disabled(Signal<bool>)` on `ButtonBuilder`** for parity with
   slider/checkbox/toggle. Buttons gating on reactive state is a
   universal pattern.

3. **Add typed sliders: `slider_int(Signal<i32>, range)`,
   `slider_u32(Signal<u32>, range)`** — or a generic `slider::<T>` —
   so apps that want integer values don't write the f32→i32 cast +
   `#![allow(cast_*)]` dance every time.

4. **Document the `.peek()` vs `.get()` rule in the prelude `Signal`
   doc-comment.** This is the single biggest reactive-framework
   footgun and it's currently invisible from the prelude.

5. **Add a `sigil::testing::tree_size(builder_fn)` helper** that wraps
   the with_cx + visit_preorder + assert_count pattern. Every
   dogfood test re-implements it.

6. **Surface `IntoNode` in the prelude doc-comment** with a one-line
   "all builders implement IntoNode; you can pass any of them to
   `.child()` or `cx.add()`." Without this hint, type errors involving
   builder returns are unreadable.

## What I'd flag for the next dogfood pass

- Workspace pedantic lints (`cast_possible_truncation`, `cast_sign_loss`,
  `struct_excessive_bools`, `fn_params_excessive_bools`) hit examples
  hard. They're correct for library code but punitive for tier-1
  utilities. Consider a `[lints]` block override for `examples/`
  members that relaxes these specifically.

- `clippy::struct_excessive_bools` (>3 bools in a struct) and
  `fn_params_excessive_bools` (>3 in fn args) forced me to refactor
  `PasswordOpts { lower, upper, digits, symbols }` into a `u8`
  bitmask. The bitmask version is fine, but the refactor was friction
  for what is genuinely a "this struct has 4 boolean flags" use case.
  Bitflags is overkill for an example app.

- No `disabled_when` reactive disable on button (see proposal #2)
  meant I dropped the "disable Generate when no class is selected"
  affordance. Real apps will want this.
