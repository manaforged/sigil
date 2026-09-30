# Fields

Text and value entry. The text widgets write their bound signal on
every edit and then call `.change()` as a side effect — you do not need
`.change()` to keep the signal current.

## text_field

`text_field(Signal<String>) -> TextFieldBuilder`. A single-line edit
with placeholder, selection, undo history, IME preedit, and clipboard
shortcuts.

```rust,no_run
# use sigil::prelude::*;
# let _ = {
let name = signal(String::new());
text_field(name)
    .placeholder("Display name")
    .submit(|value| tracing::info!(value, "submitted"));
# };
```

Key options:

- `.placeholder(impl Into<TextContent>)` — ghost text while empty.
- `.change(Fn(&str))` — runs after each committed edit.
- `.submit(Fn(&str))` — runs on Enter.
- `.blur(Fn())` — runs when the field loses focus.
- `.disabled(Signal<bool>)` — blocks input and paints disabled.
- `.invalid_when(Signal<bool>)` — paints the error frame; a failed
  clipboard write also raises it.
- `.clipboard(impl Clipboard)` — inject a clipboard; defaults to the
  installed handle (see [Plumbing](plumbing.md#clipboard)).
- `.password()` — masks the glyphs with `*`; the signal stays
  plaintext.
- `.reveal(Signal<bool>)` — shows the plaintext while true.
- `.autofocus()` — focus this field when it mounts.
- `.focus_when(Signal<u64>)` — bump the signal to focus this field
  after it mounts.
- `caret_visible()` / `scroll_offset()` — `&mut self` accessors that
  return the internal signals so you can observe or drive them.

Editing keys: Arrow Left/Right, Home, End, and Shift variants for
selection; Backspace/Delete; Ctrl (or Cmd on macOS) + C/X/V for
clipboard, A for select-all, Z for undo, Shift+Z or Ctrl+Y for redo.
IME input lands as inline preedit, and the caret rect is published to
the accessibility tree for the IME candidate window.

## text_edit

`text_edit(Signal<String>) -> TextEditBuilder`. The multi-line editor:
the text_field surface plus newlines, vertical caret movement, and
word/line editing commands.

- `.placeholder()`, `.disabled()`, `.change(Fn(&str))`,
  `.clipboard()`, `.autofocus()`, and `.focus_when(Signal<u64>)` work
  as in `text_field`. `.submit(Fn(&str))` runs on Ctrl+Enter — plain
  Enter inserts a newline.
- `.caret_position(Signal<usize>)` — bind the caret as a byte offset.
- `.caret_visible()` — accessor for the caret visibility signal.
- `.content_padding(horizontal, vertical)` — inner padding in px.
- `.chrome(bool)` — `false` drops the field frame/background.
- `.state(TextEditState)` — hand the widget a shared state object
  instead of loose signals.

`TextEditState::new(value, caret)` bundles the value signal, a caret
signal, and a selection signal. `state.select(byte, extend)` moves or
extends the caret, `state.insert(text)` inserts at the caret,
`state.replace(text)` rewrites the buffer, and `state.selection()`
returns `Option<Range<usize>>` (`None` when there is no selection
anchor). Use it when a second widget or a
script must edit the buffer programmatically.

Editing keys add: Arrow Up/Down, PageUp/PageDown, Alt+Left/Right (or
Alt+B/F) for word motion, Alt+Backspace and Ctrl+W to kill a word
backwards, Alt+D forwards, Ctrl+U/Ctrl+K to kill to line edges, and
Shift on any motion key to extend the selection.

## number_field

`number_field(Signal<T>, RangeInclusive<T>) -> NumberFieldBuilder<T>`
where `T: NumericKind` — a sealed trait implemented for `i32`, `i64`,
`usize`, `f32`, and `f64`. A text field that parses and clamps:

```rust,no_run
# use sigil::prelude::*;
# let _ = {
let quantity = signal(1_i32);
number_field(quantity, 1..=99);
# };
```

The visible buffer is a separate string signal. Edits parse on every
change; parseable input is clamped into the range and written to the
bound signal, unparseable input stays in the buffer without touching
the signal. An external `set` re-renders the buffer. Float kinds reject
non-finite input. `.disabled(Signal<bool>)` works as in `text_field`.
`.invalid_when(Signal<bool>)` paints the error frame through the
`text_field` validation state; the frame also shows while the buffer
does not parse or is outside the range. `.text_field()` consumes the
builder and returns the inner `TextFieldBuilder` so you can chain field
options after the numeric wiring.

## date_field

`date_field(Signal<Date>) -> DateFieldBuilder`. An ISO `YYYY-MM-DD`
text field over the same buffer pattern: valid parses write the signal,
invalid text only lives in the buffer, and external sets re-render.

`Date { year: i32, month: u8, day: u8 }` — `Date::new` validates month
and day including leap years, `Date::parse` reads `YYYY-MM-DD`, and
`date.render()` formats it. `.disabled(Signal<bool>)` and
`.invalid_when(Signal<bool>)` work as in `number_field`; the error frame
also shows while the buffer is not a valid date. `.text_field()`
returns the inner builder.

## color_field

`color_field(Signal<Rgba8>) -> ColorFieldBuilder`. A swatch plus a hex
text field. Accepts `#rgb` and `#rrggbb`; writes `Rgba8` with alpha 255.
The swatch is one control height square. `.disabled(Signal<bool>)`
disables the field and dims the swatch. `.invalid_when(Signal<bool>)`
paints the field error frame; it also shows while the text is not a hex
colour.
`parse_hex(&str) -> Option<Rgba8>` and `to_hex(Rgba8) -> String` are
the helpers the widget itself uses.

## combobox

`combobox(Combobox) -> NodeBuilder`. An editable text field with a
filterable popup list. Unlike the other builders, the state is a
standalone object you construct first:

```rust,no_run
# use sigil::prelude::*;
# let _ = {
let state = Combobox::new(vec!["Rust".into(), "Go".into(), "Zig".into()])
    .placeholder("Language");
combobox(state);
# };
```

`Combobox` holds the option list, the query, the selection, the popup
open flag, and the cursor. Read it back with `.query()`,
`.selected()` / `.selected_option()`, `.selected_signal()`, and
`.is_open()`; drive it with `.open()`, `.close()`, `.commit(index)`,
`.edit_query(text)`, and `.move_cursor(delta)`. `.disabled(Signal<bool>)`
disables the text field. `.filtered()` returns
the visible `(index, label)` pairs — a case-insensitive substring
match.

Typing opens the popup and clears the selection unless the text equals
the committed label. Arrow Down opens or moves the cursor, Arrow Up
moves it, Enter commits the cursor row, Escape closes. The popup also
closes when the text field loses focus. The popup caps
at eight 28 px rows and scrolls past that. Committing bumps a session
signal that rebuilds the field so the label repaints.

## dropdown

`dropdown(Signal<usize>, Vec<String>) -> DropdownBuilder`. A trigger
button plus a popup list — pick by index, not free text.

```rust,no_run
# use sigil::prelude::*;
# let _ = {
let choice = signal(0_usize);
dropdown(choice, vec!["Small".into(), "Large".into()])
    .change(|i| tracing::info!(i, "picked"));
# };
```

`.label()` names the control for accessibility (default "Dropdown");
`.disabled(Signal<bool>)`, `.width(px)`, `.height(px)` (default 32),
`.change(Fn(usize))` — runs after the signal writes.

Enter opens or commits, Escape closes, the arrows move the cursor, and
pointer enter moves it too. `focus_out` on the root closes the popup.
The popup is a `TopLayer` list of `Role::ListBoxOption` rows capped at
six visible options; longer lists scroll inside the popup.

## field

`field(label, control)` stacks a label over any control.
`.error(reactive)` adds an error line under a text, number, or colour
field. The error is a `Signal<Option<String>>` or a closure that
returns one. While it is `Some`, the field paints its invalid frame and
shows the message.

## States, keys, and accessibility

The visual catalog captures each field in the states in the table.
Invalid comes from `.invalid_when(Signal<bool>)`, from a buffer that
does not parse, or from a `field(..).error(..)`. Open is the popup
of a dropdown or combobox.

| Widget | States | Keys | Role | Name source |
| --- | --- | --- | --- | --- |
| `text_field` | default, hover, pressed, focused, disabled, invalid | Editing keys, Enter submits | `TextInput`, or `PasswordInput` with `.password()` | The placeholder, else "Text field" |
| `field(..).error(..)` | default, focused, invalid | As `text_field` | As `text_field` | As `text_field` |
| `text_edit` | default, hover, pressed, focused, disabled | Editing keys, Ctrl+Enter submits | `TextInput` | The placeholder, else "Text field" |
| `number_field`, `date_field`, `color_field` | default, hover, pressed, focused, disabled, invalid | As `text_field` | `TextInput` | As `text_field` |
| `dropdown` | default, hover, pressed, focused, disabled, open | Enter, Arrow keys, Escape | `ComboBox`, popup `ListBox` | `.label()`, else "Dropdown" |
| `combobox` | default, hover, pressed, focused, disabled, open | Typing, Arrow keys, Enter, Escape | `ComboBox`, popup `ListBox` | The placeholder, else "Combobox" |

`field` adds a visible label but does not name the control. Set the
placeholder, or `.label()` on a dropdown, so assistive technology reads
the same name.

## Terminal notes

- There is no IME in the terminal host. Paste arrives through
  bracketed paste.
- The caret is the terminal cursor: a steady bar at the caret cell.
- Copy and cut send OSC 52, so they reach the terminal clipboard.
- The dropdown and combobox popups draw above clips, after the rest of
  the tree.
- Size a field with `rows()`, not pixels. See [Units](../units.md).

## Example

Run it with `cargo run -p sigil-ui --example widget_fields`.

```rust,ignore
{{#include ../../../crates/sigil/examples/widget_fields.rs}}
```
