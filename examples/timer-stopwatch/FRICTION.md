# Friction Log — timer-stopwatch dogfood

App: a press-to-record stopwatch (start / stop / lap / reset) with a
monospaced elapsed-time display and a lap list. Built on `sigil::launch`.

---

## Time-to-first-paint estimate

~45 minutes from copying `hello-counter/Cargo.toml` to a `cargo check`-clean
binary. Most of that was reading source — the build itself is fast. The
friction breakdown below explains where the reading time went.

---

## Discovered by grepping source

These are things I could NOT learn from the prelude doc-comment or the
`hello-counter` example alone. Every one required opening a source file.

**1. `text()` accepts `Signal<String>` directly via `From` impl.**
I expected to need `derived(|| signal.get())` everywhere and pass a `String`.
Found the `From<Signal<String>> for TextContent` impl only after reading
`crates/kernel/src/text.rs:43`. The hello-counter example already uses this
pattern (`text(label_text)`) but the prelude doc-comment does not call it out.
The fact that `text()` accepts both static strings AND reactive signals via
the same `impl Into<TextContent>` is the most important ergonomic fact about
the text primitive, and it is invisible without reading source.

**2. `derived` returns `Signal<T>`, not a separate `Derived<T>` wrapper.**
`crates/reactive/src/derived.rs:114`. I expected a newtypes distinction;
the unified `Signal<T>` return means you can pass a `derived(...)` anywhere
a `Signal<String>` fits with no adapter.

**3. `font-mono` is a valid class string parsed to `StyleAtom::FontFamily("mono")`.**
Found at `crates/style/src/parser.rs:72,166`. The prelude doc-comment lists
`font-mono` in the utility-class table but does not explain that it maps to
`StyleAtom::FontFamily` rather than a size or weight token. This matters
because I initially assumed "font-mono" was a theme token name, not a
family selector.

**4. `Cx::new_root()` is the headless construction path.**
Found by reading `examples/hello-tree/src/main.rs:9`. Not mentioned in the
prelude or the `sigil::launch` doc. There is no `Cx::headless()` or
`Cx::test()` alias — you just call `Cx::new_root()`. Fine once found, but
zero discoverability from the public surface.

**5. `signal()` must be called inside a reactive scope.**
Not documented anywhere visible. The test `tree_builds_without_panic` panicked
with a cryptic "Owner not found" message when `signal()` was called at the top
of the test function. The fix — wrapping in `sigil::reactive::Runtime::scope("name", || { ... })` — was discovered by the team-lead from reading
`crates/reactive/src/owner.rs`. There is no panic message that says "call
`Runtime::scope` first" and no example shows this pattern for tests.

**6. `sigil_kernel` is NOT a declared dep; use `sigil` re-exports instead.**
My test module imported `use sigil_kernel::prelude::Cx;` because `hello-tree`
does that. But `timer-stopwatch` only declares `sigil = { workspace = true }`,
which re-exports `Cx` via `sigil::prelude::*`. The test compiled fine in
isolation but the dep was undeclared. Fixed by the team-lead to
`use sigil::prelude::Cx;`. The correct import path is non-obvious because
`sigil_kernel` appears in every other example but the sigil facade crate
intentionally hides sub-crate names.

---

## Missing widgets / primitives

**No interval / animation-tick hook.**
A stopwatch where the display only updates on button press is a
"press-to-record" log, not a live stopwatch. To show a rolling `MM:SS.mmm`
display, an app needs a per-frame or N-ms callback that can call
`elapsed_ms.set(...)`. There is no such hook in the public API. I had to
design around the absence: the display text is derived from `elapsed_ms`
which is only updated at Stop/Lap time, so the display freezes while running.
The FRICTION is that this design decision is forced by the framework, not
chosen for the app.

`FctSpawn` and `dispatch_to_owner` exist in the prelude (`crates/sigil/src/lib.rs:98,49`)
but their relationship to a repeating timer is not obvious. A
`sigil::launch_with_tick(Duration, fn)` or a `cx.interval(Duration, fn)`
primitive would unblock an entire category of real-time apps.

**No numeric input widget.**
There is `text_field(signal)` for string input. Parsing numeric values requires
the caller to attach a `.change(|s| { if let Ok(n) = s.parse::<f64>() { ... } })`
handler and manage parse errors manually. For a unit-converter app this would
be the dominant boilerplate. A `num_field::<f64>(signal)` builder with
built-in parse and format would eliminate it.

**No label/value pair layout primitive.**
The pattern "label on left, value on right" (e.g., "Lap 1: 00:01.234")
is unavoidable in utility apps but requires manual flex-row construction
every time. Even a simple `kv_row(label, value)` builder function would
reduce friction in this category.

---

## Confusing names

**`signal(x)` vs `Signal::new(x)`.**
Both exist (functions vs type). `signal(x)` is the right call in reactive
scope (`crates/reactive/src/signal.rs`). Found the function form in
`hello-counter/src/main.rs:18`. The prelude exports the function under
`sigil_reactive::signal` and the type under `sigil_reactive::Signal`.
No confusion once seen, but the dual exposure is worth documenting.

**`peek()` vs `get()`.**
`peek()` reads the signal value without tracking it as a dependency.
`get()` reads and subscribes. In a `derived(move || ...)` closure, you
almost always want `get()`. In an event handler, you almost always want
`peek()`. Nothing in the prelude or the type signature tells you which to
use where — you have to read `crates/reactive/src/signal.rs` and understand
the dependency-tracking model before this distinction makes sense. The
`hello-counter` example uses both in adjacent lines without comment:
`count.set(count.peek() + 1)` — the asymmetry is correct but opaque.

**`container()` vs `panel()`.**
`container()` is a layout primitive. `panel()` is a widget with themed
background + border. For someone coming from a CSS-box mental model,
"container" implies a styled card, not a raw flex/block box. I initially
reached for `panel()` for the outer wrapper and then had to read both
builders to understand the distinction.

---

## Boilerplate I had to copy-paste

- The six workspace `[package]` keys (`version.workspace = true`, etc.) — copied
  verbatim from `hello-counter/Cargo.toml`. There is no template or `cargo
  generate` hook, so every new example starts with a copy-paste ritual.
- `#![allow(missing_docs)]` — every example needs this because the workspace
  `[workspace.lints.rust]` enables `missing_docs = "warn"`. Examples are not
  library code; the allow should be default for the `examples/` subtree or
  there should be a blanket `[profile.examples]` lint override.
- `LayoutStyle::flex_column().fixed_size(w, h).gap(n)` — repeated on every
  root container in every example I read. The `sigil::launch(("title", w, h), tree)`
  call already knows the logical size; there should be a way to let the root
  node inherit that size automatically rather than restating `fixed_size` in
  the tree builder.

---

## Things that surprised me (good)

**`derived` composition is clean.** Chaining two derived signals
(`display_text = derived(|| format_ms(elapsed_ms.get()))` and
`status_text = derived(|| if start_at.get()...`)`) just works. No explicit
subscriptions to wire, no manual invalidation. The derived graph is invisible
and correct.

**Class strings are expressive.** `"bg-surface text-on-surface text-xl font-mono"`
reads like Tailwind and works. No import needed, no token lookup — the parser
accepts a space-separated string and maps it to atoms. The prelude doc-comment
listing all valid class tokens is genuinely useful (rare for internal docs).

**`button("label").primary().activate(|| { ... })`** — the cascade is exactly
what you want. One line, no intermediate variables, no builder struct exposed.
The `activate` closure captures signals by copy without any Arc/clone boilerplate.

**`Signal<T: Copy>` is `Copy` itself.** Passing `start_at` and `elapsed_ms`
into multiple closures without `.clone()` is frictionless. Many reactive
libraries force you to `Arc::new(Mutex::new(...))` to share state across
handlers; here you just pass the signal by value.

---

## What I'd change if I owned this API

**1. Add `Runtime::test_scope(name, f)` as a one-liner for test setup.**
The current pattern is `Runtime::scope("name", || { ... })`. Wrapping
`signal()` calls in this is not intuitive and the panic message on failure
("Owner not found") does not point at the fix. A `sigil::testing::test_scope(f)`
that sets up the minimal reactive context + cleans up after the closure would
make headless tests self-contained without reading internals.

**2. Add a tick/interval hook to `sigil::launch`.**
Something like:
```rust
sigil::launch_with_tick(("title", w, h), Duration::from_millis(100), app_tree)
```
where the shell drives a regular signal update that apps can opt into. Without
this, any real-time display (clock, progress, live metrics) requires reaching
outside the framework to a thread + channel, which breaks the reactive model.

**3. Expose `num_field::<T>(signal: Signal<T>)` as a first-class widget.**
Numeric input is the second most common widget after `text_field`. The current
workaround (parse in `.change()`) loses cursor position on invalid input and
produces jittery re-renders. A typed numeric field with configurable format
and validation would remove the most common boilerplate in utility apps.

**4. Let `sigil::launch` propagate the window size to the root node automatically.**
Today: `sigil::launch(("title", 480, 480), app_tree)` AND
`container().layout(LayoutStyle::flex_column().fixed_size(480.0, 480.0))`.
The size is stated twice. If `launch` injected a root-size signal into the
`Cx`, the tree builder could call `.layout(LayoutStyle::flex_column().fill())`
and resize correctly on window scale changes too.

**5. Add a workspace `[package.metadata.sigil-example]` marker so `cargo run --example`
works without a `[[bin]]` section.**
Right now each example needs either a `[[bin]]` or the package name must match
the binary name exactly. The `hello-counter` package name _is_ `hello-counter`
so `cargo run -p hello-counter` works. But the convention is invisible —
nothing in the README or the `examples/` directory explains why you use
`-p <name>` instead of `--example <name>`. A brief `examples/README.md` entry
per app with the exact invocation would halve the first-run friction.
