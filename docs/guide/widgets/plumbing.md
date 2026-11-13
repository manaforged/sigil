# Plumbing

The low-level pieces the widgets are built from: drag mechanics, clipboard, and terminal wiring. Use them when a stock widget does not fit. Most app code never needs them directly.

## Interactive nodes

Any `NodeBuilder` is an interactive element. `.hover()`, `.pressed()`, `.focus()`, and `.disabled_paint()` style it per state, and the node tracks its own hover, press, and focus. `.track_hovered(signal)`, `.track_pressed(signal)`, and `.track_focused(signal)` write that state into a `Signal<bool>` for code that needs it.

- `.activate(f)`: run `f` on a primary click, Enter, or Space. The node takes focus and shows the focus ring.
- `.disabled(value, signal, or closure)`: dim the node, ignore its pointer and keys, and report it as disabled.
- `.focusable(true)`: take keyboard focus without an action.

`button`, `card`, `list_row`, and the menu-bar triggers are built this way. If you only need event handlers, use the `Widget` event methods (`pointer_down`, `key`, …) directly.

## axis_drag

`axis_drag(node, Orientation::Horizontal | Orientation::Vertical, snapshot, on_delta, on_end) -> NodeBuilder`. On pointer down it captures the pointer and calls `snapshot()` to grab your baseline value; each captured move calls `on_delta(baseline, delta_px)`; release calls `on_end()`. Splitter handles are built on it. Use it for any drag-to-adjust control. For a drag on both axes, `Widget::pointer_drag(|phase, event| ...)` captures the pointer and calls the handler with `DragPhase::Start`, `Move`, `End`, or `Cancel`; `event.prevent_default()` in the `Start` call skips the drag.

## drag and drop

`DragState::<T>::new()` is a shared session object for one drag type; `T: Clone`.

```rust,no_run
# use sigil::prelude::*;
# use sigil::widgets::{DragState, drag_ghost};
# let node = container();
# let bin = container();
# let payload_string = String::from("task:1");
# let _ = {
let drag = DragState::<String>::new();
// source:
node.drag_source(drag.clone(), payload_string);
// target:
bin.drop_target(drag.clone(), |item| item.starts_with("task:"), |event| {
    let _at = event.local_position();
    let _item: String = event.into_value();
});
// floating preview, mounted once at the root:
drag_ghost(drag, |item| text(item.clone()).nowrap());
# };
```

- `node.drag_source(state, payload)` (from `Widget`) arms on left-button press and begins the drag after 4 px of travel.
- `node.drop_target(state, accepts, on_drop)` registers the node's live bounds; while a payload it accepts hovers, the target gets the class `bg-accent-4`, and release calls `on_drop` with a `sigil::events::DropEvent<T>`. `event.value()` and `event.into_value()` return the payload, `event.position()` is the drop point in window coordinates, `event.local_position()` is the drop point relative to the target, and `event.bounds()` is the target's `Rect`.
- `node.drop_hover(state, cursor)` on a drop target writes the target-local pointer position into a `Signal<Option<Point>>` while an accepted payload hovers the node, and `None` otherwise. Use it to draw an insertion marker. A target with `drop_hover` does not get the `bg-accent-4` class.
- `drag_ghost(state, render)` returns a non-hittable node that follows the pointer while a drag is active.
- `state.is_active()`, `state.position()` (a `Point`), `state.payload()`, and `state.cancel()` expose the session; `cancel` aborts without dropping.

## clipboard

`trait Clipboard { fn set(&self, &str) -> Result<Delivery, Error>; fn get(&self) -> Option<String>; }` in `sigil::clipboard`. A thread-local handle backs it: `install(clip)` installs, `clipboard()` returns the `Rc<dyn Clipboard>`. By default `set` errors and `get` returns `None` until something installs a real handle.

`sigil::run` calls `install(OsClipboard)` when the `desktop` feature (`clipboard` → arboard) is on, so text widgets paste from the OS automatically. Outside `run` (tests, embed hosts), nothing is installed. `sigil::clipboard::copy(text)` / `paste()` are the `AppResult` convenience wrappers, and the `clipboard-image` feature adds `image()`. `text_field` accepts a per-widget `.clipboard(clip)` override.

## terminal

`terminal(lines: Signal<Vec<String>>, on_input: Fn(String)) -> NodeBuilder`. A dumb pipe view, not an emulator: it paints `lines` as monospace rows in a `virtual_list` pinned to the bottom, and forwards key-down input as terminal bytes: typed text verbatim, Enter as `\r`, Backspace as `DEL`, Tab as `\t`, Escape, the arrows, Home, End and Delete as their escape sequences, and Ctrl+letter as its control byte. The node is a focusable `Role::Terminal`, and nothing inside it takes focus. While it has focus it receives every key, Tab included, and the window's handlers and shortcuts see only keys held with the command key.

Pair it with `sigil::app::Session` (feature `pty`): `Session::spawn(program)` opens a PTY (`Session::spawn_in(program, dir)` starts it in `dir`), parses output through vt100 into `session.lines` (a `Signal<Vec<String>>`), and `session.write(&bytes)` sends input to the child. Spawn requires a `Owner::scope` owner. `Session` exists only with the `pty` feature. Dropping the session kills the child. The `terminal` widget itself is not feature-gated. Feed it lines from anywhere.

For a full-screen program, use `session.view()` on an `Rc<Session>` instead of `terminal`. The view fills its space and resizes the terminal to the cells that fit, so the program sees the new size. It paints colors from the active theme: the 16 ANSI colors, the 256-color palette and truecolor map to the gray, error, warn, success, info and accent scales. It shows bold, italic, inverse and the cursor, and takes paste (bracketed when the program asks for it) and the wheel (5000 rows of scrollback). `session.resize(cols, rows)`, `session.scroll(rows)` and `session.paste(text)` do the same from code. The child starts with `TERM=xterm-256color`.

## States, keys, and accessibility

The visual catalog captures an interactive node in default, hover, pressed, focused, and disabled, and `terminal` in default, hover, pressed, and focused.

| Piece | Keys | Role | Name source |
| --- | --- | --- | --- |
| Interactive node | Enter and Space with `.activate` | The node's role | The node |
| `axis_drag` | Pointer only | The node's role | The node |
| `drag_source`, `drop_target` | Pointer only | The node's role | The node |
| `terminal` | Every key-down is sent to `on_input` | `TextInput` | None |

Drag and drop has no keyboard path. Offer a menu item or a shortcut that makes the same change.

## Terminal notes

- Drag and drop works in the terminal host: SGR mouse reports a press, motion, and release at the cell centre. The pointer moves one cell at a time.
- The `terminal` widget is a view of lines. It works in both hosts. A `Session` needs the `pty` feature.
- The clipboard in the terminal host is OSC 52 for writes. Reads come through bracketed paste.

## Example

Run it with `cargo run -p sigil-ui --example widget_plumbing`.

```rust,ignore
{{#include ../../../crates/sigil/examples/widget_plumbing.rs}}
```
