# Chat-client dogfood — friction log (v2)

Tier-5 kitchen-sink app: `virtual_list` + `splitter` + `tabs` + `text_field` + `text_edit`, plus a `modal` over the lot. Single new-dev pass, ~1h 20min wall clock from `cargo new`-equivalent to green gate (no actual `cargo new` — I copied the kanban-board shape almost verbatim).

## Time-to-first-paint

**v1 baseline (per the dispatch brief):** 3.5 hours for the tier-5 kitchen-sink kanban-board.

**v2 measured:** ~1h 20min on chat-client. Most of that win is the kanban example existing — `BoardState` is a copy-paste template for the `Copy`-carrier pattern, and the splitter / modal / tabs invocations transplanted with zero API friction. Pure greenfield (no kanban to crib from) I'd estimate ~2h, mostly spent re-discovering that `text_edit` is in `sigil::widgets::` but not in `sigil::prelude` (see Discovery below).

The bottleneck is no longer "what does the API look like" (the prelude is reasonable). It is "which container layout combinator do I need to make the splitter actually grow to fill the parent." Layout debugging is opaque without a tree inspector.

## Discovered by grepping source

Things I had to grep the framework for, with `file:line` citations:

1. **`text_edit` is not in the prelude.** `prelude.rs:98-107` re-exports `text_field` but not `text_edit`. Had to `use sigil::widgets::text_edit;` explicitly. The widget exists at `crates/sigil/src/widgets/text_edit/builder.rs:52`. Adding it to the prelude is a one-line change. **Counts as a regression vs v1's expectation** — v1's brief told me to look in `crates/sigil/src/widgets/text_edit/`, implying the v2 prelude would have caught it up. It hasn't.

2. **`SelectionMode` is in the prelude but the path `sigil::widgets::SelectionMode` also works.** `prelude.rs:103-107` re-exports `SelectionMode`. I initially wrote the fully-qualified path matching what kanban-board does (`sidebar.rs:26`). Both are fine, but the kanban example sets a precedent for `sigil::widgets::SelectionMode::Single` over `SelectionMode::Single`, so I followed it. Minor noise — neither is wrong.

3. **`splitter` is also dual-pathed.** Prelude has `SplitterOrientation`, `SplitterBuilder`, `splitter` but kanban uses `sigil::widgets::splitter(sigil::widgets::SplitterOrientation::Horizontal)`. I matched the kanban style. Same friction.

4. **`TabsBuilder` is not `NodeBuilder`** — it `IntoNode`s but cannot be chained into another `.child()` plus `.layout()` on the same builder. I wrote a one-trait `TabsPassthrough` to wrap it in `container().child(tabs_builder)` (`chat_pane.rs:38-51`). This is the kind of thing that should be a method on `TabsBuilder` — `into_container() -> NodeBuilder` — so the conversion is fluent.

5. **`text_edit` has `.height(...)` via `WidgetLayoutExt`** — `widgets/layout_ext.rs`. Took a `grep -r "fn height" crates/sigil/src/widgets/`. Not obvious from the prelude doc which methods are widget-author-facing vs layout-override.

6. **`modal()` panics at build time if `.show(signal)` isn't called.** `widgets/modal/builder.rs:158-161`. The doc on `modal()` mentions `.show(...)` but I had to read the `into_node_builder` source to realize the panic is a hard assertion, not a soft default. Documented; fine in practice; would be nicer as a type-state where `show` is required by the type system.

7. **`Cx::first_child(root)` is the test-side handle to walk the kernel tree.** Discovered from `kanban-board/src/tests.rs:20` since `with_cx` itself doesn't expose tree walkers. Probably ought to be reachable via the `sigil::testing::` module rather than the kernel-level `Cx` directly.

## Cross-widget composition ergonomics

Modal-over-tabs-over-splitter-over-virtual_list-with-text_edit composed cleanly. The key pattern is:

- splitter sits at the top of the work area, sized via flex_row + grow(1.0)
- splitter's left child = sidebar (virtual_list of channels)
- splitter's right child = chat_pane (tabs)
- tabs' content_for(MESSAGES) = container holding virtual_list of messages + compose_box (text_edit + send button)
- modal is a sibling of the work area, gated by `state.modal_open`

Concrete friction points:
- The splitter sizing relies on `sizes_bind(signal)` for state, but the per-index `min_size(0, ...)` / `max_size(1, ...)` reads as a mini-DSL where you have to remember which index is which child. A `.child_constrained(node, initial, min, max)` would be more honest.
- `tabs.content_for(idx, node)` is great. **Strong + carry**: this is exactly the API I'd want.
- `text_edit.placeholder("Type a message — Enter for newline, Ctrl+Enter to send.")` — but the placeholder doesn't include keyboard hints anywhere reactive. Users will not discover `Ctrl+Enter` from a static placeholder. Tooltip + keyboard-hint surface is a missing widget.

## State management

5 interacting widgets, all signals on a `Copy` carrier struct (`state.rs:46`). This is the pattern that makes Sigil tractable. Without `Signal<T>: Copy`, every handler would need an `Rc<RefCell<_>>` per closure and the wiring would be unreadable.

The pain: `state.channels.peek().get(idx).map(|c| c.messages.clone())` is the standard "snapshot the current channel for build" idiom and shows up in every right-pane tab (messages_tab / members_tab / pinned_tab) (`chat_pane.rs:55-60`, `90-97`, `123-130`). A `state.current_channel() -> Option<Channel>` helper that does this once would cut ~15 lines.

The bigger ergonomic gap: **when the selected_channel signal changes, the tab content does NOT rebuild automatically.** I read the channel snapshot at build time inside each tab body. Switching channels updates `selected_channel`, the status bar (bound to `state.status`) re-renders, but the message list shows the previous channel's messages until the next full tree rebuild. **For this dogfood pass I left this as-is** (and called it out in the modal-rebuild test) but a production chat app would need reactive `virtual_list` children — same gap text_edit has with reactive line children.

## Missing widgets / primitives (the biggest gaps)

These hurt the most in v2 (v1 surfaced different gaps; tracking what STILL hurts):

1. **Reactive children adapter.** `text_edit`'s line layout is a build-time snapshot (`widgets/text_edit.rs:15-32`). Same problem hits any virtual_list whose source signal mutates after build. Workaround: rebuild the whole tree on commit. Production-grade tier-5 apps need a `.children_reactive(derived(|| ...))` on `NodeBuilder`.

2. **Tooltip + keyboard-hint surface.** Discoverability of `Ctrl+Enter` (text_edit submit), `Esc` (modal close), `Enter` (text_field submit), `Tab/Shift-Tab` (focus traversal) is zero. A `.hint("Ctrl+Enter to send")` adapter on text_edit + a `Hints` overlay widget would be high-leverage.

3. **Scroll-to-bottom on virtual_list.** Chat UX requires "show latest" — the `virtual_list` has no `.initial_scroll(Bottom)` or `.follow_signal(latest_index)`. New messages get added off-screen.

4. **Avatar / icon widget.** Author names in message_row are bare labels. Even a 16px circle with initials would make the example look like a chat app rather than a styled list. Sigil has `sigil-icons` but it's not in the prelude.

5. **Inline confirm action.** Send button uses `button("Send").primary().activate(...)`. No way to express "this is a destructive / commit-style action; show pending state during the network call." Async-tied button states are a gap.

6. **Form / fieldset wrapper.** The modal has two labeled text_fields. I open-coded `label("Title") + text_field(...)` pairs. A `field(label, control)` helper would dedupe ~6 lines per modal.

## Confusing names

- `splitter` vs `tabs` vs `panel` — three different things, but a fresh reader of `prelude.rs:98-107` can't tell which is "split this region into two children" vs "switch between three views" vs "general grouping." Doc-strings on the prelude re-exports would help.
- `.label(...)` on `tabs` is the ARIA label; `label(text)` is a widget that renders text. Two senses of the same word in adjacent lines (`chat_pane.rs:31` vs `chat_pane.rs:64`).
- `SelectionMode::Single` vs `SelectionState` — both in the prelude. The mode is the policy, the state is the data. Fine once you know, surprising on first read.
- `dismiss_on_backdrop(false)` is the default for modal but `escape_closes(true)` is the default too. The defaults are inconsistent in spirit — one is "user-blocking" the other "user-friendly."

## Boilerplate copy-paste

What I copy-pasted from kanban-board, by section:
- `BoardState` → `ChatState`: same shape (Copy carrier + signals + open/commit/cancel modal + select_X methods). **~80 lines of copy-paste pattern, zero behavior change.** A `#[derive(SigilState)]` macro that gives me `seed`, modal helpers, and selection helpers from a struct of `Signal<T>` fields would cut a chunk.
- `root_shell` / `top_bar` / `work_area` / `status_bar`: identical 4-section layout. Could be a `app_shell(top, work, status, modal)` helper in the prelude.
- The `modal_body` + Cancel/Save `actions()` row: identical between kanban edit_modal and chat settings_modal. Could be `modal_form(...).action("Cancel", ...).action("Save", ...).primary()`.

Net: the example is ~440 LOC. ~140 LOC of that is reusable pattern that should live in `sigil::widgets::patterns`.

## Surprised-good

- **`Signal<T>: Copy` is the linchpin.** Every handler is a clean `move ||` closure. No `Rc`, no `Arc`, no `Mutex`. This is the reason Sigil is actually usable for kitchen-sink apps; without it the wiring would be unreadable.
- **`launch(("Sigil - chat-client", 1100, 720), app_tree)`** — tuple-as-config for the common case. I expected to need `AppConfig::new().title(...).size(...)`. The tuple is exactly the right ergonomic shortcut.
- **`with_cx("name", |cx| { ... })`** — the test helper. Both smoke tests and the new send_compose test wrote themselves once I had the pattern. 5 lines, no boilerplate.
- **Modal `escape_closes(true)` stacks correctly.** I didn't have to wire any key handlers myself — the modal builder's static `ESCAPE_MODALS` stack (`widgets/modal/builder.rs:246-280`) handles the topmost-modal-first semantics. Good default.
- **`virtual_list(items, |idx, item| row(...))`** — closure-per-row API is clean. The fact that the closure receives `(idx, &T)` rather than just `&T` is exactly what I needed for `state.open_edit(col, idx)`-style handlers.
- **`.class("w-full rounded-md bg-surface-hover")`** is the Tailwind-style sweet spot — utility classes for the 80% case, `.style(...)` builder for the 20%. The prelude doc enumerates the accepted atoms (`prelude.rs:79-89`), which I actually read and used. Good move.

## API change proposals

1. **Add `text_edit` to the prelude.** One-line change to `crates/sigil/src/prelude.rs:98-107`. The kanban example already exports it via `sigil::widgets::*` — the prelude should match. **Highest impact / lowest cost.**

2. **`TabsBuilder::into_container() -> NodeBuilder`** — fluent unwrap. Today I have to write a passthrough trait (`chat_pane.rs:38-51`). Would let me write `tabs(...).content_for(...).into_container().class(...)` end to end.

3. **`NodeBuilder::children_reactive(derived_signal)`** — accept a `Derived<Vec<impl IntoNode>>` and re-emit children when the derived value changes. Closes the text_edit reactive-children gap and unblocks chat / live-feed UI in general.

4. **`virtual_list::initial_scroll(ScrollAnchor::Bottom)` and `::follow(signal)`** — chat / log UX. Currently a virtual_list of 40 messages starts at index 0 with no way to jump to the last entry.

5. **`SplitterBuilder::child_constrained(node, initial, min, max)`** — collapses the per-index `child(node, init).min_size(idx, ...).max_size(idx, ...)` triplet to one fluent call. Removes the index-tracking gotcha.

6. **`sigil::widgets::patterns::app_shell(top, work, status, modal)`** — the four-section layout I copy-pasted from kanban. Saves ~40 LOC per kitchen-sink example and gives a documented reference shape.

7. **`field(label_text, control_builder)`** — wraps a label + a text_field/text_edit/checkbox in a vertical pair. Saves the 4-line `container().child(label("X")).child(text_field(s))` pattern that every form duplicates.

8. **`button("X").activate_async(|| async { ... })` with pending state.** Today there's no way to mark a button as "in flight" while a handler runs. Chat apps, file uploads, anything network-bound needs this.
