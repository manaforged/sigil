# Plumbing

The low-level pieces the widgets are built from: interaction state,
drag mechanics, clipboard, and terminal wiring. Use them when a stock widget does not fit — most app code
never needs them directly.

## interactive

`interactive(inner: NodeBuilder) -> InteractiveBuilder` turns any node
into a state-tracking widget: it installs internal `hovered`,
`pressed`, and `focused` signals and resolves the `Style` state
overlay against them, so `.hover()`, `.pressed()`, `.focus()`, and
`.disabled_style()` style closures work on a plain container.

- `.focusable(bool)` — participate in keyboard focus.
- `.disabled_signal(Signal<bool>)` — drive the disabled slot.
- `.class(&str)` — base classes; `.into_node_builder()` finishes.

This is the primitive `button`, `icon_button`, and the menu-bar
triggers are made of. If you only need event handlers, use
`NodeEventsExt` (`pointer_down`, `key`, …) directly — `interactive` is
for full state styling.

## axis_drag

`axis_drag(node, Axis::X | Axis::Y, snapshot, on_delta, on_end) ->
NodeBuilder`. On pointer down it captures the pointer and calls
`snapshot()` to grab your baseline value; each captured move calls
`on_delta(baseline, delta_px)`; release calls `on_end()`. Splitter
handles are built on it — use it for any drag-to-adjust control.

## drag and drop

`DragState::<T>::new()` is a shared session object for one drag type;
`T: Clone`.

```rust,no_run
# use sigil::prelude::*;
# let node = container();
# let bin = container();
# let payload_string = String::from("task:1");
# let _ = {
let drag = DragState::<String>::new();
// source:
node.drag_source(drag.clone(), payload_string);
// target:
bin.drop_target(drag.clone(), |item| item.starts_with("task:"), |item| { /* ... */ });
// floating preview, mounted once at the root:
drag_ghost(drag, |item| text(item.clone()).nowrap());
# };
```

- `node.drag_source(state, payload)` (from `DragDropExt`) arms on
  left-button press and begins the drag after `DRAG_THRESHOLD_PX`
  (4 px) of travel.
- `node.drop_target(state, accepts, on_drop)` registers the node's live
  bounds; while a payload it accepts hovers, the target gets
  `DROP_TARGET_CLASS` (`bg-accent-4`), and release calls `on_drop`.
- `drag_ghost(state, render)` returns a non-hittable node that follows
  the pointer while a drag is active.
- `state.is_active()`, `state.position()`, `state.payload()`, and
  `state.cancel()` expose the session; `cancel` aborts without
  dropping.

## clipboard

`trait Clipboard { fn set(&self, &str) -> Result<(), ClipError>; fn
get(&self) -> Option<String>; }` in `sigil::clipboard`. A thread-local handle backs it:
`install(clip)` installs, `clipboard()` returns the `Rc<dyn
Clipboard>`. The default is `NoopClipboard` — `set` errors and `get`
returns `None` until something installs a real handle.

`sigil::run` calls `install(OsClipboard)` when the `desktop` feature
(`clipboard` → arboard) is on, so text widgets paste from the OS
automatically. Outside `run` — tests, embedded hosts — nothing is
installed. `sigil::clipboard::copy(text)` / `paste()` are the
`AppResult` convenience wrappers, and the `clipboard-image` feature
adds `image()`. `text_field` and `text_edit` accept a per-widget
`.clipboard(clip)` override.

## terminal

`terminal(lines: Signal<Vec<String>>, on_input: Fn(String)) ->
NodeBuilder`. A dumb pipe view, not an emulator: it paints `lines` as
monospace rows in a `virtual_list` pinned to the bottom, and forwards
key-down input as strings — typed text verbatim, Enter as `\n`,
Backspace as `DEL`, Tab as `\t`. The node is a focusable
`Role::TextInput`.

Pair it with `Session` (`sigil::app::Session` or via the prelude under
`pty`): `Session::spawn(program)` opens a PTY, parses output through
vt100 into `session.lines` (a `Signal<Vec<String>>`), and
`session.write(&bytes)` sends input to the child. Spawn requires a
`Runtime::scope` owner. `Session` exists only with the `pty` feature. Dropping the session
kills the child. The `terminal` widget itself is not feature-gated —
feed it lines from anywhere.

## States, keys, and accessibility

The visual catalog captures `interactive` in default, hover, pressed,
focused, and disabled, and `terminal` in default, hover, pressed, and
focused.

| Piece | Keys | Role | Name source |
| --- | --- | --- | --- |
| `interactive` | None of its own. Add `.key()` handlers on the inner node. | The inner node's role | The inner node |
| `axis_drag` | Pointer only | The node's role | The node |
| `drag_source`, `drop_target` | Pointer only | The node's role | The node |
| `terminal` | Every key-down is sent to `on_input` | `TextInput` | None |

Drag and drop has no keyboard path. Offer a menu item or a shortcut
that makes the same change.

## Terminal notes

- Drag and drop works in the terminal host: SGR mouse reports a press,
  motion, and release at the cell centre. The pointer moves one cell at
  a time.
- The `terminal` widget is a view of lines. It works in both hosts.
  A `Session` needs the `pty` feature.
- The clipboard in the terminal host is OSC 52 for writes. Reads come
  through bracketed paste.

## Example

Run it with `cargo run -p sigil-ui --example widget_plumbing`.

```rust,ignore
{{#include ../../../crates/sigil/examples/widget_plumbing.rs}}
```
