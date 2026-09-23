# Plumbing

The low-level pieces the widgets are built from: interaction state,
drag mechanics, clipboard, terminal and audio wiring, and floating
combat text. Use them when a stock widget does not fit — most app code
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

```rust,ignore
let drag = DragState::<String>::new();
// source:
node.drag_source(drag, payload_string);
// target:
bin.drop_target(drag, |item| item.starts_with("task:"), |item| { /* ... */ });
// floating preview, mounted once at the root:
drag_ghost(drag, |item| label(item.clone()));
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
get(&self) -> Option<String>; }`. A thread-local handle backs it:
`install(clip)` installs, `clipboard()` returns the `Rc<dyn
Clipboard>`. The default is `NoopClipboard` — `set` errors and `get`
returns `None` until something installs a real handle.

`sigil::launch` calls `install(OsClipboard)` when the `desktop` feature
(`clipboard` → arboard) is on, so text widgets paste from the OS
automatically. Outside `launch` — tests, embedded hosts — nothing is
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
`Runtime::scope` owner and the `pty` feature — without the feature
`spawn` returns `Err("enable feature `pty`")`. Dropping the session
kills the child. The `terminal` widget itself is not feature-gated —
feed it lines from anywhere.

## transport and Player

`transport(playing: Signal<bool>, progress: Signal<f32>, toggle: Fn())
-> TransportBuilder`. A play/pause `icon_button` (⏸/▶ glyphs, labeled
"Play or pause") beside a `slider` bound to `progress` over `0.0..=1.0`
at 0.01 steps. `.seek(Fn(f32))` wires the slider's `.change` to a seek
callback — it fires on release, not continuously.

`Player` is the matching state object: public `playing` and `progress`
signals, plus `toggle()`, `tick()` (syncs `progress` from the sink —
call it on an interval), `seek(ratio)`, and `play_path(path)`. Real
playback needs the `audio` feature (rodio); without it `seek` and
`play_path` return `Err("enable feature `audio`")` and `toggle` is a
no-op.

## floating_text

`floating_text(spawn: Signal<FctSpawn>) -> FloatingTextBuilder`.
Game-style floating combat text: write a `FctSpawn` to the signal and a
particle rises and fades.

- `FctSpawn::new(at: [f32; 2], text, color)` — spawn point, text, tint.
- `.duration(seconds)` — particle lifetime.
- `.drift([dx, dy])` — travel over the lifetime.
- `.time(Signal<f32>)` — the clock; drive it from a frame/tick signal
  so floaters animate.
- `.at(Signal<(f32, f32)>)` — anchor all floaters to a moving point.
- `.style(FloaterStyle)` — `Damage` (error-11), `Heal` (success-11), or
  `Miss` (gray-12) overrides the spawn color.
- `.capacity(n)` — pool size, default `FCT_POOL_CAPACITY` (1024);
  spawns beyond the pool are dropped.

For manual control, `FctState` is the standalone pool (`new`,
`new_anchored`, `spawn`, `tick`, `for_each_alive`,
`alive_count`, a `generation` signal), and `FctNode` mounts an existing
`Rc<FctState>` into the tree.
