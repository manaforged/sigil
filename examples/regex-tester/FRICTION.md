# Regex tester — friction log

Tier-3 dogfood (text mutation + persistence-shaped state). Two `text_field` inputs (pattern + sample), a derived `Result<Vec<MatchInfo>, String>`, and a rendered match-list block plus a status label. ~310 LOC including 10 tests.

## Time-to-first-paint

About 40 minutes from `cd examples/` to green `cargo test`. ~70% of that was clippy whack-a-mole + discovering that `text_field` does not implement `.class()`. The reactive design + view tree both took shape in the first ~10 minutes once I had `scratchpad/main.rs` open as a template; the bulk of the time was lint-loop and discovering missing surface.

Time-to-first-paint for the actual `cargo build` (interactive launch): **not attempted**. The dispatch was headless-only — `cargo build` succeeded and `cargo test` ran the smoke `tree_builds_without_panic`, but I did not run `cargo run -p regex-tester` interactively from this dispatch shell. See "Honesty" at end.

## Discovered by grepping source (file:line)

Things I had to find by reading the crate rather than the prelude / public docs:

- `sigil::widgets::text_edit` / `TextEditBuilder` — `crates/sigil/src/widgets.rs:49`. **NOT** in the prelude (`crates/sigil/src/lib.rs:98-107` only re-exports `TextFieldBuilder`, not `TextEditBuilder`). To use multi-line input you must reach into `sigil::widgets::text_edit`, which is a second namespace beyond `sigil::prelude::*`. Discoverability cost: I had to grep for "text_edit" across the workspace to learn it existed at all.
- `text_edit` module doc — `crates/sigil/src/widgets/text_edit.rs:10-32`. The "**Multi-line rendering — IMPORTANT limitation**" block (line-children freeze at build time). This is critical for any editor-tier app and only visible if you open the module file. Should be surfaced in the prelude rustdoc or in a top-level `docs/` page.
- `sigil::Result<T>` — `crates/app/src/error.rs:7`. The prelude exports `Result` as a **single-arg** alias `Result<T> = core::result::Result<T, sigil::Error>`. The first time I wrote `fn scan(...) -> Result<Vec<MatchInfo>, String>` the compile error was `E0107: type alias takes 1 generic argument but 2 generic arguments were supplied`. Confusing because `Result` is "obviously" the std `Result` to a new reader. I had to type `core::result::Result<_, _>` explicitly for non-sigil-error returns.
- `text()` factory — `crates/kernel/src/text.rs:58`. Distinct from `label()`. `label()` is in `sigil::widgets::shared`; `text()` is in `sigil_kernel`. The distinction (label vs text node) was not obvious — I learned from `file-browser/src/right_pane.rs:58` which uses `text(body)` for the derived signal body.
- `text_field` builder methods — `crates/sigil/src/widgets/text_field/builder.rs`. Only `.placeholder(...)`, `.disabled(...)`, `.change(...)`, `.submit(...)`, `.clipboard(...)`, `.caret_visible(...)`, `.scroll_offset(...)` and the `WidgetLayoutExt`/`StateHost`/`StyleAccess` traits. **No `.class()`** — I tried `.class("font-mono")` and got `E0599 no method named class found for struct TextFieldBuilder`. `button`, `checkbox`, `scroll`, `tabs`, `tooltip`, `toggle`, `slider` all have inherent `.class()`, but text_field doesn't. (See "Missing widgets / primitives" below.)
- `Runtime::scope("name", || { … })` — `sigil::reactive::Runtime`. Required to wrap `Signal::set` calls inside tests. Discovered from `scratchpad/src/main.rs:132`. Not in the prelude.

## `text_field` / `text_edit` ergonomics — specifics

### `text_field` (the one I actually used)

**Signal binding shape.** `text_field(signal_of_string)` — the `Signal<String>` is passed by value (it's `Copy` because signals are handle-shaped). Reading the bound value at any point is `state.pattern.get()` (subscribes) or `state.pattern.peek()` (non-subscribing snapshot). This is clean — the signal IS the binding, no `bind_text` / `bind_value` builder method needed. **Surprised-good.**

**`.change` vs `.submit`.**
- `.change(|s: &str| ...)` fires after every keystroke. I did NOT need this for the regex-tester because the derived signal already re-runs on every `pattern.set()` / `sample.set()` triggered by the widget's internal write.
- `.submit(|s: &str| ...)` fires on Enter for `text_field`, on Ctrl+Enter for `text_edit`. I also did not need this; clear is a separate button.

So for this app I bound nothing explicit — the `text_field` writes back into the bound signal, the derived signal recomputes, the view re-renders. This is exactly the reactive model I wanted and it Just Worked. **Surprised-good #2.**

**Focus / IME.** Not exercised — I did not run interactively this dispatch, and the smoke tests are headless tree-build only.

**Multi-line behaviour.** I chose `text_field` (single-line) over `text_edit` (multi-line) deliberately. `text_edit`'s module doc (`crates/sigil/src/widgets/text_edit.rs:10-32`) calls out:

> **the visible line layout is a snapshot of the buffer's line count at widget-build time**, not a live reactive view of the buffer.

For a regex tester this would have been actively misleading — the user types `\n` (e.g. in the sample text) and the value updates but the visible row count doesn't grow. The sample text is *typically* multi-line, but the limitation means it stays single-line until rebuild. So I stuck with single-line `text_field` and documented the choice in the module doc.

The text_edit regression test (`crates/sigil/tests/text_edit.rs:114-143`) pins this exact behaviour. The fix lives in a follow-up (reactive-children adapter on `NodeBuilder`).

### What didn't work / minor cuts

- **`.class("font-mono")` on `text_field` doesn't compile.** Other widgets (button, tabs, etc.) have inherent `.class()`. text_field/text_edit don't, even though they're THE one widget that most needs monospaced styling. I had to drop the class entirely; my "Pattern" field renders in proportional font.
- **No prelude re-export of `Runtime`.** `Runtime::scope` is required to wrap any test that calls `Signal::set` — without it, you get a panic about no reactive runtime. The discoverability path is "grep for Runtime in other examples." Should be in `sigil::prelude::*`.

## Missing widgets / primitives

- **Code-editor-shaped widget.** A real multi-line text widget with reactive-line-count and monospace defaults. `text_edit` is half of it (the buffer rope is right) but the static line snapshot makes it unusable as-is for any editor app.
- **`.class()` on `text_field` / `text_edit`.** As above — no styling escape hatch except `WidgetLayoutExt::width/height/grow/...`. There's no way to set `font-mono`, `text-sm`, or any visual class on a text input.
- **A reactive-list widget.** `virtual_list` takes `items: Vec<T>` by value at build time (see `crates/sigil/src/widgets/virtual_list/mod.rs:74-95`). The items are NOT a `Signal<Vec<T>>` — appending to the list requires rebuilding the widget. For a regex tester I want "the match list grows/shrinks as the user types" — that's exactly the use case `virtual_list` *looks* like it should solve, but the API takes a snapshot. I worked around it with a single derived `text()` block (one match per line). This is the single biggest editor-tier API gap I hit.
- **A `text_field` that takes `Signal<bool>` for "is valid"** so the field can paint a red border on regex compile error. Right now I surface the error in a sibling label; an inline red-border treatment would be one line of declarative style if a `.invalid_when(signal_bool)` existed.

## Confusing names

- **`Result`** — re-exported from `sigil` as a single-arg alias. Trips up the first time you try to write `Result<T, E>` for non-sigil errors. Either rename to `AppResult` / `SigilResult` or drop the re-export entirely and force `sigil::Result` qualification.
- **`label` vs `text`** — `label(signal_of_string)` is the widget; `text(signal_of_string)` is the kernel-level text node. Both accept `impl Into<TextContent>`. The distinction matters (label is a widget that wraps text + a11y label semantics) but the naming gives no hint. A new user picks one at random.
- **`Role::Main` / `Role::GenericContainer` / `Role::ListItem`** — these are accesskit names (`pub use accesskit::Role` at `crates/kernel/src/lib.rs:37`). `GenericContainer` is the catch-all and the name is awkward. I used `Role::Main` for the root and `Role::GenericContainer` for the results panel; this is exactly the inherited-from-accesskit awkwardness `file-browser/FRICTION.md` already noted.
- **`activate(|| ...)` on `button`** vs **`submit(|s| ...)` on `text_field`** — both are "user-completed-the-thing" callbacks but the names diverge. `submit` takes `&str`, `activate` takes nothing. Consistent naming (`click` / `on_click` / `pressed` would be more conventional) would help.

## Boilerplate copy-paste

- **`container().layout(LayoutStyle::flex_column().gap(N))` + `.child(label("Foo"))` + `.child(input)`** — every form row is this exact 4-line shape. A `form_row("Pattern", text_field(...))` helper would collapse 3 of my rows to one line each.
- **`derived(move || match scan(...) { Ok(...) => fmt_a, Err(_) => "" })`** twice — once for the status label, once for the match body. Both wrap the same `scan()` call. The fix is to derive `scan_result: Signal<Result<Vec<MatchInfo>, String>>` once and then derive both labels from THAT signal. I didn't because `Result<..., String>` isn't trivially `Clone`-cheap and I wanted to avoid the noise — but the cleaner version is what an experienced Sigil dev would write.
- **Test harness `Runtime::scope("name", || { ... })`** boilerplate in every reactive test. A `#[sigil::test]` attribute macro (or a `with_runtime("name", || ...)` helper in `sigil::testing`) would remove 2 lines per test.
- **`Signal<String>` → `String::from("…")` for `.set()`** every time. `signal.set("…")` (taking `&str` or `impl Into<String>`) would shave one `String::from(...)` per call site.

## Surprised-good

- **Derived signals integrate into `label(signal)` and `text(signal)` with zero ceremony.** I write `let result_label = derived(move || ...)` returning `Signal<String>`, hand it to `label(result_label)`, and the label re-renders on every dependency change. No `subscribe`, no `bind`, no manual `effect`. This is THE thing the reactive model should be good at, and it is.
- **`Signal: Copy`**. State structs that hold 4 signals can derive `Clone, Copy`. Closures captured by `move ||` don't need explicit clones. This is huge — it removes the entire class of "I forgot to clone before the move" friction other reactive frameworks have.
- **`Regex::new(...) -> Result<_, _>`** is the right shape; no panic path. The error message returned by the regex crate is human-readable and went straight into the label.
- **Compile-time hook discipline.** Every Edit/Write triggered clippy with `-D warnings` + fmt. I was annoyed at the moment (5 round-trips before green) but every issue was real: dead `class()` calls, doc backticks, `unwrap_err` in tests, `single_char_pattern`. The hook caught what code review would have caught later. **The strictest configuration I've worked under, and the friction is honest friction.**

## 3-5 concrete API change proposals

1. **Re-export `TextEditBuilder` and `text_edit` from `sigil::prelude::*`.** The single line `pub use crate::widgets::{TextEditBuilder, text_edit, …}` in `crates/sigil/src/lib.rs:98-107` (alongside `TextFieldBuilder, text_field`). Right now any editor app forces a second namespace import `use sigil::widgets::text_edit;` and that's gratuitous.

2. **Add inherent `.class(impl Into<ClassSource>)` to `TextFieldBuilder` and `TextEditBuilder`.** Every other interactive widget (`button`, `checkbox`, `scroll`, `tabs`, `tooltip`, `toggle`, `slider`) has it. text_field and text_edit are the ONLY widgets that don't, and they're the ones where styling matters most (font-mono for code-ish input).

3. **Wire reactive items into `virtual_list`.** Either `virtual_list(items_signal: Signal<Vec<T>>, render)` or a `.items_bind(signal)` builder method. Today the items are captured by value at build time, which makes any "list that grows in response to user input" use case impossible without manual rebuild. This is the single biggest gap an editor-tier app hits.

4. **Drop the single-arg `sigil::Result<T>` re-export, or rename it.** It shadows std `Result` and produces a confusing `E0107: type alias takes 1 generic argument but 2 generic arguments were supplied` the first time a user writes `Result<T, E>` for a non-sigil-error API. Rename to `AppResult<T>` or require qualified `sigil::Result<T>` only.

5. **Add `Runtime` to `sigil::prelude::*`** (or expose a `sigil::testing::with_runtime("name", || ...)` helper). Right now every reactive test in every example writes the same `Runtime::scope("test-name", || { … })` wrapper, found via grep. The `with_cx` helper exists already (`crates/sigil/src/testing.rs`) for widget-tree tests; the same shape for pure-state tests would remove the boilerplate.

## Honesty

I did NOT launch the app interactively this dispatch. The verification gate is:
- `cargo check -p regex-tester` → green
- `cargo build -p regex-tester` → executable produced
- `cargo clippy -p regex-tester --all-targets -- -D warnings` → clean
- `cargo test -p regex-tester` → 10 passed; 0 failed

The reactive-state tests cover the actual model behaviour (`result_label` updates on pattern change, `match_body` updates on sample change, `clear` resets both). The widget tree builds without panic under `Runtime::scope`. I cannot claim from this dispatch that the UI looks right or that text input works in the live window — `interactive_launch: headless only` in the structured return.
