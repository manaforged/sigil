# Kanban Board — Sigil Tier-5 Dogfood Friction Log

**App:** kanban-board  
**Tier:** 5 (full app — 5 widgets exercised simultaneously)  
**Date:** 2026-05-20  
**Widgets exercised:** modal, tabs, splitter, virtual_list, text_field

---

## Time to first compiling build

~3.5 hours wall-clock (single experienced developer, no prior Sigil exposure).  
Breakdown: ~30 min reading the framework, ~2 h fighting API surface gaps and lint conflicts, ~1 h wiring state + smoke tests.

---

## Bugs discovered

### 1. `TextFieldBuilder::on_submit` does not exist — correct method is `.submit()`

**Severity:** blocks every form-submission pattern.  
**Where:** the Sigil vertical-slice example at `examples/vertical-slice/src/main.rs` (not examined inline, but the compilation error surfaced it).  
The builder method is `.submit(move |val| { ... })`, not `.on_submit(...)`. The discrepancy cost ~20 min.

### 2. `text_edit` is `pub(crate)` — completely invisible from downstream

**Severity:** moderate — developers discovering `text_edit` by grepping will write code that fails to compile.  
**File:line:** `crates/sigil/src/widgets/text_edit.rs:1` — module-level visibility is not re-exported.  
The public multiline-edit widget is `text_field`. `TextEdit` is the internal cursor/rope struct. No public alias or re-export exists.

### 3. `splitter` and `SplitterOrientation` are not in the prelude

**Severity:** moderate friction — every file importing splitter must write the full path.  
`use sigil::widgets::splitter;` and `use sigil::widgets::SplitterOrientation;` are both required.  
**File:line:** `crates/sigil/src/lib.rs` re-exports block does not include `splitter` or `SplitterOrientation`.

### 4. `virtual_list` is not in the prelude

**Severity:** same as above.  
`use sigil::widgets::virtual_list;` required explicitly.  
`SelectionMode` is also absent from the prelude.

### 5. `VirtualListBuilder<T, R>` cannot be used as a `NodeBuilder` child directly

**Severity:** high — composition gap requires a workaround shim.  
`virtual_list(items, render_fn)` returns `VirtualListBuilder<T, R>` which implements `IntoNode`, but `NodeBuilder::child()` expects something that satisfies the `IntoNode` bound in a specific way that requires wrapping in a `container()` call.  
**Workaround:** added a `VirtualListShim` trait with one impl that wraps the builder in a passthrough `container()` and calls `.child()` internally. ~15 lines of boilerplate per use-site avoided, but the trait itself is boilerplate.

### 6. `modal()` panics at build time if `.show()` is not called

**Severity:** footgun — the panic message is a bare `assert!` with no contextual text.  
A `ModalBuilder` without a `.show(signal)` call will `assert!`-panic when the tree is built. The error does not point to the call site. Expected: compile-time error or a descriptive runtime message.

### 7. Workspace lint conflict: `unreachable_pub` vs `redundant_pub_crate` in binary crates

**Severity:** moderate — both lints are enabled at workspace level; they conflict in `[[bin]]` crates.  
`unreachable_pub` fires on every `pub` item in a binary crate (nothing is exported).  
Changing all `pub` → `pub(crate)` then triggers `redundant_pub_crate` from the workspace nursery lints.  
**Resolution:** `#![allow(clippy::redundant_pub_crate)]` at crate root. The workspace lint configuration should special-case binary crates or provide a `[profile.bin-lints]` escape.

### 8. `&String` in `Fn(usize, &T)` closure triggers `clippy::ptr_arg`

**Severity:** minor but surprising.  
When `T = String`, the `render_fn` closure for `virtual_list` receives `&String`. Clippy fires `ptr_arg: use &str instead of &String` on the closure parameter — correct in isolation, but the closure signature is constrained by the `Fn(usize, &T)` bound. Requires careful deref-coercion at the call site or a type annotation. Not obvious on first encounter.

---

## Ergonomics friction

### Cross-widget composition is verbose at the modal layer

Composing `modal()` inside a `splitter()` inside `tabs()` requires each layer to be explicit about the passthrough container. The modal needs to live at the root of the tree (or at least outside the split region) to cover the full viewport, but `splitter()` takes ownership of its children. The workaround — a zero-layout `container()` sibling to the splitter at the top level — works but is not obvious.

### `tabs().content_for(index, node)` API is correct but undiscoverable

The API is good. The discoverability is not: the method `content_for` is not mentioned in any top-level example. Developers reaching for tabs will likely try `.child_for_tab()`, `.panel()`, or `.tab_content()` first.

### Splitter sizes are `Signal<Vec<f32>>` — init requires matching length

`splitter()` with N children requires `Signal<Vec<f32>>` of exactly N values. If the length mismatches, behaviour is undefined (no panic, just wrong layout). Expected: a builder-level assertion.

---

## What worked well (good surprises)

1. **`Signal<T>` is `Copy`.** This makes `BoardState` — a struct carrying many `Signal<T>` fields — trivially `Copy`. All event handlers are plain `move ||` closures with no `Rc<RefCell<_>>` anywhere in the app. This is a genuinely great DX choice.

2. **`tabs()` with `content_for(index, node)` routing.** Inactive panels get `Display::None` automatically. No manual `if active_tab == N` branching needed. Felt natural after the first use.

3. **`modal().escape_closes(true)` just works.** Keyboard escape dismissal is a single flag. No event-handler wiring required.

4. **`text_field(signal).submit(closure)` is ergonomic.** Once you know the method name is `.submit()` (not `.on_submit()`), form submission is a two-liner.

5. **`BoardState` as a flat Copy signal carrier** scales cleanly. A single `state: BoardState` parameter flows through every pane/component function. No threading complexity, no borrow issues.

---

## Missing widgets (blockers for real apps)

| Widget | Impact |
|---|---|
| `text_area` (multiline) | Cannot build any card-description editor |
| `select` / `dropdown` | No column-picker, no filter-by-status |
| `icon_button` | Toolbar looks text-only; no affordances |
| `context_menu` | Right-click on card — no hook |
| `toast` / `snack_bar` | No transient feedback after save |
| `scrollable` with momentum | Needed for column-list on mobile/touch targets |
| `form_layout` | Label-input alignment requires manual row-containers |
| `progress_indicator` | No loading state for async card saves |
| `badge` / `chip` | Tag display on cards |
| Modal `min_width` / `max_height` config | Modal sizes to content; can't cap or constrain |

---

## API change proposals

1. **Add `splitter`, `SplitterOrientation`, `virtual_list`, `SelectionMode` to `sigil::prelude`.**  
   All four are in every non-trivial Sigil app. The extra `use sigil::widgets::*` import is friction with no benefit.

2. **`VirtualListBuilder<T, R>` should implement `Into<NodeBuilder>` or `NodeBuilder::child()` should accept `IntoNode` directly.**  
   Eliminates the shim workaround.

3. **`ModalBuilder::show()` should be enforced at compile time, not runtime.**  
   Either return a different `PendingModal` type that only resolves to `NodeBuilder` after `.show()`, or make the show-signal a constructor argument: `modal(show_signal)`.

4. **Rename `text_edit` internal module or add a `#[doc(hidden)]` guard** so it doesn't appear in IDE autocompletion next to `text_field`.

5. **Provide a `#[sigil::bin_lints]` attribute or a workspace lint profile** for binary crates to opt out of `unreachable_pub` without triggering `redundant_pub_crate`.

6. **`text_field` builder: alias `.on_submit()` to `.submit()` with a deprecation notice**, or rename to `on_submit` consistently. Current name is inconsistent with the "event handler = `on_*`" convention used elsewhere.

7. **`tabs().content_for()` should appear in the top-level widget gallery** with a three-tab example. It is the most ergonomic part of the tabs API and the hardest to discover.

8. **`splitter()` should assert at build time that `sizes.len() == children.len()`** with a descriptive message.

---

## Boilerplate that should be framework-provided

- `VirtualListShim` trait (~20 lines) — one per app, needed every time `virtual_list` appears inside a `container`.
- Passthrough `container()` wrapper for `modal()` at the tree root (~5 lines).
- The `#![allow(clippy::redundant_pub_crate)]` lint suppress in every binary crate.

Total boilerplate tax for a new developer hitting these three for the first time: approximately 45 minutes of confusion + ~35 lines of non-app code.
