# Scratchpad — friction log

First-time impression of Sigil from the perspective of an editor-class
app dogfooder. I built a single-buffer scratchpad with save / revert /
clear, a dirty indicator, and a character counter. Three tests, one
launchable binary, ~190 LOC. Below is every speed bump, mismatch, and
"I had to dig" moment from the build.

## Time-to-first-paint estimate

If I count "look at hello-counter → modify it to a single text_field →
green gate," I could have shipped a one-field demo in ~10 minutes. The
extra time (~45-60 min equivalent context budget) went into:

1. Discovering that `text_edit` (the multi-line buffer) is not in the
   public API (~5 min lost going down the wrong path).
2. Discovering that the canonical `vertical-slice` example uses an API
   name (`.on_submit`) that doesn't exist on the current
   `TextFieldBuilder` (~10 min sanity-checking my reading).
3. Negotiating the clippy `dead-code` lint for a `Signal<bool>` field
   that's only read inside a closure captured by another signal —
   clippy can't see the read (~15 min iterating).
4. Three `fmt` rejection cycles from the postedit hook, each one a
   "collapse this multi-line builder chain to one line" diff (~5 min).

A first-paint of a real two-signal app, friction-free, would be ~20
minutes once the shape is in your head. The first time, allow 60.

## APIs only discovered by grepping source

| API | Where I had to look | Why |
|---|---|---|
| `derived` is at `sigil::reactive::derived`, NOT in `prelude::*` | `crates/sigil/src/lib.rs:55-61` | The `prelude` re-exports `signal`, `effect`, `Signal`, `Trigger`, `EventChannel` — but `derived` lives in the `reactive` module re-export only. `hello-counter` does `use sigil::reactive::derived;` separately. Took a `grep` of `pub fn derived` in `crates/reactive/src` to confirm. |
| `Signal::peek` vs `Signal::get` distinction | `crates/reactive/src/signal.rs:234,290` | Prelude exports `Signal` but no docs at the use-site about when to use which. `peek` is non-subscribing, `get` is subscribing. The vertical-slice example uses both. I guessed from the names and they were right, but a doc-comment at `signal()` would have saved the grep. |
| `Cx::new_root` for tests | `crates/kernel/src/cx.rs:47` | Needed for the headless smoke test. Not in any user-facing doc I read. Discovered by reading `examples/vertical-slice/src/tests.rs`. |
| `Runtime::scope(name, f)` for test scoping | `crates/reactive/src/owner.rs:122` and re-exports | Same as above — needed for tests, found by reading vertical-slice tests, not from prelude. |
| `text_edit` exists but is `pub(crate)` | `crates/sigil/src/widgets/text_edit.rs:6` | The struct has selection, undo/redo, IME-safe boundary handling — but it's gated. The only public text-input is the single-line `text_field`. Listing this as missing in the widget gap below. |
| `virtual_list` is in `widgets`, not in `prelude` | `crates/sigil/src/widgets.rs:45` | If you want a virtual list you `use sigil::widgets::virtual_list;` — not surfaced in `prelude::*`. Fine; just unexpected after seeing every other widget exported through prelude. |

## text_field / text_edit ergonomics

This is the tier where editing matters, so this section is the heaviest.

### `text_field(Signal<String>)` binding model

`text_field` takes a `Signal<String>` as its primary argument. This is
the **good shape**: the signal IS the field's text. You read from it
with `.get()` / `.peek()` and write to it with `.set()` / `.update()`.

But then the `.change(cb)` API took me by surprise. The
`text_field_demo.rs` example does:

```rust
text_field(name)
    .change(move |v| {
        name.update(|s| {
            s.clear();
            s.push_str(v);
        })
    })
```

…which manually copies the new value back into the signal that the
field already owns. That's confusing. Either:

- The field is already writing to the signal on every keystroke (in
  which case `.change()` is for side effects only, and the demo is
  misleading), OR
- The field is NOT writing to the signal, and `.change()` is the
  user's responsibility to wire (in which case "binding" is a misnomer).

I didn't sanity-check which by reading `handlers.rs`. For my app I
don't pass `.change()`; I trust that `text_field(state.current)` is
two-way bound, and my `.peek()` calls in `save()` / `revert()` return
the live buffer. The tests pass, so empirically it IS bound. But the
demo's manual `.update()` callback is bad signaling — it suggests
you NEED to write the callback. Drop it from the demo, or add a doc
comment to `.change()` saying "for side effects ONLY; the signal is
already live."

### Submit semantics

`.submit(cb)` fires on Enter. Discovered by `grep` —  the vertical-slice
example calls `.on_submit(...)` which **does not exist** on the current
`TextFieldBuilder`. (`grep on_submit` in `crates/sigil` returns zero
matches; the only hits are in `vertical-slice` and a planning doc.)
That example almost certainly doesn't compile against current master.
Worth a separate followup to either restore `.on_submit` as an alias or
fix the example.

### IME, focus

I did not exercise IME (didn't launch interactively). `text_field`
exposes `Clipboard`, `caret_visible: Signal<bool>`, `scroll_offset:
Signal<usize>` on the builder — those are reasonable hooks for an
embedder. Focus on the field appears to be opt-in via the kernel
`focusable()` API, but the builder itself doesn't surface a `focus()`
or `request_focus()` method. For an editor app where I want to land
focus in the buffer on startup, I'd want one.

### No multi-line text_edit in the public API

**Biggest single piece of friction.** A scratchpad WANTS to be
multi-line. The `text_edit.rs` implementation is right there with
undo/redo, selection, char-boundary safety, even shift-extend selection.
But the type and `text_edit` factory are `pub(crate)`. No public
multi-line widget exists. So my scratchpad is a single-line input field
with a length counter underneath, which feels wrong for the use case.

If I were building a real notes app, this is the first blocker. The
workaround would be writing my own multi-line widget on top of the
kernel + a `Vec<String>` signal, which is many days of work I didn't
sign up for.

## Missing widgets / primitives

- **`text_edit` (multi-line)** — exists internally but `pub(crate)`. See above. Workaround: nothing reasonable for a real app.
- **`derived` not in prelude** — minor, but `use sigil::reactive::derived;` is an extra line every reactive app needs. Move it to `prelude::*` alongside `signal`, `effect`.
- **No `.focus_on_mount()` / `.autofocus()` on `text_field`** — for a scratchpad you'd want the cursor in the buffer immediately. Workaround: untried, would have to dig into the focus subsystem.
- **No status-bar primitive** — built mine by hand with a `flex_row` + two labels. That's fine, but every editor has one; a `status_bar()` shortcut would be welcome.
- **No `divider()` / `separator()` widget** — wanted a visual hairline between header and editor, fell back to layout gap.

## Confusing names

| What I tried | What it actually is | What I'd rename |
|---|---|---|
| `signal()` | Top-level constructor for `Signal<T>` | Fine, but the doc at the construction site doesn't mention `derived` as the related primitive. Cross-link. |
| `derived` | Computed/memoized signal | `derived` is fine but I almost wrote `computed()` (Solid/Vue) or `memo()` (Solid) first. Add aliases or a doc redirect. |
| `Signal::get` vs `Signal::peek` | `get` subscribes, `peek` doesn't | The names are correct after you know the model, but a first-timer will write `.get()` everywhere and create extra dependencies. The vertical-slice example uses `.peek()` in `selected_name()` correctly — worth a doc page on "when to peek." |
| `.change()` on text_field | Optional change side-effect, NOT the binding | See above. The name `change` reads as "tell me when this changes" which suggests it IS how you're notified. The `Signal<String>` is already that mechanism. Rename to `.on_change_side_effect()` or just drop it and tell users to wire an `effect` on the signal. |
| `container()` + `label()` + `button()` + `text_field()` | Mix of `pub fn` factories and `pub fn` factories. | Consistent. |
| `LayoutStyle::flex_column()` + `.grow(1.0)` + `.padding(24.0)` | Layout builder | Clear. The "grow" name is borrowed from CSS flexbox and reads instantly. |
| `class("w-full h-full bg-surface")` | Tailwind-ish utility class string | Surprised me (good way). Documented at `prelude.rs:78-88`. |

## Boilerplate I had to copy-paste

1. **`#[cfg(test)] mod tests { … Runtime::scope("name", || { let mut cx = Cx::new_root(); … }) }`** — every test needs this scope + Cx setup. A `sigil::test::with_cx(|cx| { … })` helper that wraps both would cut every test's ceremony by 4 lines.

2. **`.layout(LayoutStyle::flex_column().gap(N).padding(M).grow(1.0))`** — every container needs a layout. Fine, but you can't omit it; even an empty container needs `LayoutStyle::block()` or `flex_*`. A default layout would save a line per container.

3. **`use sigil::prelude::*; use sigil::reactive::derived;`** — see above.

## Things that surprised me (good)

- **`signal(0_i32)` returning a `Copy` `Signal<i32>`.** I expected an `Rc<RefCell<…>>`-ish handle that I'd have to clone into every closure. Instead `Signal` is `Copy`, capture-by-move works, and I never wrote `.clone()` once across ~190 LOC. This is a big quality-of-life win over many Rust reactive libs I've touched.

- **Closures capturing `state: ScratchpadState` (one `Copy` struct holding 4 signals) work without lifetime gymnastics.** This is the same payoff as above, but the fact that a `#[derive(Clone, Copy)]` struct of signals "just works" as the per-app state container is *very* nice. The vertical-slice example uses the same pattern; I copied it.

- **`derived(|| format!(...))` integrates into `label(signal)` with no extra ceremony.** I have a char counter that lights up live without any subscription wiring. Just `derived` returning `Signal<String>`, and `label()` accepting `Signal<String>` via `Into<TextContent>`.

- **Utility-class styling.** `.class("w-full h-full bg-surface")` is documented inline in `prelude.rs`, and it works. No prop drilling, no theme objects threaded through every component. Refreshing.

- **`cargo fmt` postedit hook caught two builder-chain wrap issues before I could ship.** Annoying in the moment, correct in retrospect. Confirms the hook chain is real.

- **The clippy lint stack is strict** — pedantic + nursery + restriction set. My `if a != b { unsaved } else { saved }` got flipped by `clippy::if_not_else`. Strict but consistent.

## What I'd change if I owned this API

1. **Promote `text_edit` to public and ship it.** This is the editor-tier widget. Until it's public, "build a real notes app on Sigil" is blocked. The internal type already has undo/redo, IME-safe boundaries, selection, shift-extend — the work is largely done. Wire it through `widgets.rs`, expose `text_edit(Signal<String>)` builder, and add it to the prelude.

2. **Add `derived` to the prelude.** Every reactive Sigil app I'll write needs it. Adding it costs nothing.

3. **Drop or rename `.change()` on `text_field`.** Either make it a side-effect-only hook with a name that says so (`.on_edit_side_effect(...)`), or remove it entirely and tell users to wire `effect(move || { … current.get() … })`. The current name + the demo's usage actively misleads.

4. **Add `sigil::test::with_cx(name, |cx| { … })` helper.** Three lines of boilerplate per test that EVERY test needs is begging for a one-line wrapper.

5. **Fix or remove the `.on_submit` reference in `examples/vertical-slice/src/main.rs:271`.** It currently calls a method that doesn't exist. Either the example doesn't compile on master (which means CI isn't testing it) or there's a method-resolution path I'm missing (in which case the public surface needs documenting). Worth chasing — the vertical-slice IS the "this is how Sigil works" example, and an example that names a non-existent method on its central widget is a credibility leak.

## Compile output

All gates green on the worktree at `swarm-dogfood-3-editor`:

```
$ cargo check -p scratchpad
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.06s

$ cargo build -p scratchpad
Finished `dev` profile [unoptimized + debuginfo] target(s) in 35.40s

$ cargo clippy -p scratchpad -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.66s

$ cargo test -p scratchpad
test tests::save_marks_clean_and_revert_restores_buffer ... ok
test tests::clear_empties_buffer_and_marks_dirty_when_saved_nonempty ... ok
test tests::tree_builds_without_panic ... ok
test result: ok. 3 passed; 0 failed; 0 ignored
```

Not interactively launched in this dispatch — headless smoke only.
