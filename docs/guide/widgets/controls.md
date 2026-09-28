# Controls

Action and value controls. Each constructs from a lowercase function and
chains a builder. `button`, `icon_button`, `checkbox`, `toggle`, and
`slider` take `.class()`, `.style()`, `.theme()` on the builder;
`radio_group` reaches styling through `StyleAccess`. `button`,
`icon_button`, `checkbox`, `toggle`, and `radio_group` take the
`WidgetLayoutExt` sizing methods (`.width()`, `.height()`, `.grow()`,
`.padding()`, `.layout()`); `slider` sizes through its style classes.
`button`, `checkbox`, and `slider` implement `StateHost` — the state
closures `.hover()`, `.focus()`, `.pressed()`, and `.disabled_style()`.

## button

`button(label) -> ButtonBuilder`. `label` is `impl Into<TextContent>`:
a `&str`, `String`, `Signal<String>`, or `TextContent::compute(f)`.

```rust,no_run
# use sigil::prelude::*;
# let _ = {
let count = signal(0_i32);
button("Increment").primary().activate(move || {
    count.set(count.peek() + 1);
});
# };
```

Key options:

- `.primary()` — solid fill in the primary tone. `.neutral()` (the
  default) is a soft neutral with a frame. `.destructive()` is solid in
  the error tone. `.ghost()` drops the frame entirely.
- `.tone(Tone)`, `.solid()`, `.soft()` — set tone and variant directly.
- `.size(Size)` — Compact, Default, Wide change height and padding;
  `Size::Grow` makes the button fill its width at default height.
- `.width(px)` — pin the width; the default hugs the label.
- `.activate(Fn())` — the click/activation handler.
- `.enabled(bool)` — static disable; the node is not focusable.
- `.disabled(Signal<bool>)` — reactive disable.
- `.pending(Signal<bool>)` — ignores activation and leaves the focus
  order while true, and shows a spinner before the label. The label
  keeps its full colour; only `.disabled()` paints the disabled state.
  Activation is blocked while either is true.
- `.accessible(name)` — assistive name when the label is a glyph.

Activation fires on left-button release while the press began inside
and the pointer is still over, and on Enter or Space while focused. The
node is `Role::Button`, focusable, and carries a keyboard focus ring.
`button`, `icon_button`, `text_field`, and `dropdown` share one focus
ring: the `FocusRing` that `InteractionState` installs.

`icon_button(glyph, activate)` is `button(glyph).ghost().activate(f)`
with a square hit area: the width equals the control height and the
horizontal padding is zero. It has the button radius and the ghost
hover, pressed, focus, and disabled states. The label is a Unicode glyph. Chain
`.accessible("Play")` so the glyph gets an assistive name.

## checkbox

`checkbox(Signal<bool>) -> CheckboxBuilder`. A bordered box with a
vector check mark. `Role::CheckBox`; publishes `checked` to the
accessibility tree.

```rust,no_run
# use sigil::prelude::*;
# let _ = {
let agreed = signal(false);
checkbox(agreed).label("Accept terms").change(|on| {
    tracing::info!(on, "checkbox");
});
# };
```

Key options: `.label(impl Into<TextContent>)` puts a label beside the
box and makes the row fill its width; `.tone(Tone)` — `Tone::Neutral`
checks in gray scale step 9, every other tone checks in `accent`;
`.change(Fn(bool))` is a side effect — the widget writes the signal
first, then calls the closure; `.disabled(Signal<bool>)` blocks input
and paints the whole row, box and label, at the disabled opacity. A
disabled checked box keeps its tone fill.

Pointer release, Enter, and Space flip the value. In the terminal host
the indicator degrades to `[x]` / `[ ]` text.

## toggle

`toggle(Signal<bool>) -> ToggleBuilder`. A sliding switch:
`Role::Switch`, a 36×20 px track with a thumb. Same surface as
checkbox: `.label()`, `.disabled(Signal<bool>)`, `.change(Fn(bool))`,
and the same disabled look: the row dims and the thumb keeps its side,
and the same pointer/Enter/Space activation. The accessible name is the
label text or "Toggle".

## radio_group

`radio_group(Signal<String>) -> RadioGroupBuilder`. A vertical stack of
options bound to a string key.

```rust,no_run
# use sigil::prelude::*;
# let _ = {
let mode = signal(String::from("auto"));
radio_group(mode)
    .option("auto", "Automatic")
    .option("manual", "Manual")
    .tone(Tone::Primary);
# };
```

`.option(key, label)` appends a row (`Role::RadioButton`, publishes
`checked`). Arrow Down/Up move the selection with wraparound; Home and
End jump to the first and last option. The group is `Role::RadioGroup`,
takes one focus slot, and draws the focus ring on the checked row.
`.disabled(Signal<bool>)` blocks pointer and key selection, removes the
group from the focus order, and paints it at the disabled opacity.

## slider

`slider(Signal<f32>, RangeInclusive<f32>) -> SliderBuilder`. A drag
thumb on a filled track. `Role::Slider`; publishes the value, the
range, and the step so assistive tech can issue Increment/Decrement.

```rust,no_run
# use sigil::prelude::*;
# let _ = {
let volume = signal(50.0_f32);
slider(volume, 0.0..=100.0)
    .step(5.0)
    .change(|v| tracing::info!(v, "volume"));
# };
```

Key options:

- `.step(f32)` — quantizes every write; default 1.0, non-positive or
  non-finite steps fall back to 1.0.
- `.change(Fn(f32))` — fires on drag release and on each key step, not
  continuously during the drag.
- `.orientation(Orientation::Vertical)` — default Horizontal.
- `.length(px)` — fixes the track extent; the default fills the parent.
- `.disabled(Signal<bool>)` — dims the slider; the thumb stays opaque
  with a gray border so it reads in light and dark themes.

Keys: Arrow Right/Up add one step, Arrow Left/Down subtract,
PageUp/PageDown move ten steps, Home/End jump to the range ends. Drag
captures the pointer, so a slide that leaves the track still updates.

## States, keys, and accessibility

The visual catalog captures each control in the states in the table.
Hover, pressed, and focused come from the pointer and the keyboard.
Disabled comes from `.disabled(Signal<bool>)`.

| Widget | States | Keys | Role | Name source |
| --- | --- | --- | --- | --- |
| `button` | default, hover, pressed, focused, disabled, pending | Enter, Space | `Button` | The label, or `.accessible(name)` |
| `icon_button` | default, hover, pressed, focused, disabled | Enter, Space | `Button` | `.accessible(name)`; without it, the glyph |
| `checkbox` | default, hover, pressed, focused, disabled | Enter, Space | `CheckBox` | A static `.label()`, else "Checkbox" |
| `toggle` | default, hover, pressed, focused, disabled | Enter, Space | `Switch` | A static `.label()`, else "Toggle" |
| `radio_group` | default, hover, pressed, focused, disabled | Arrow keys, Home, End | `RadioGroup`, rows `RadioButton` | `.label()` or `.labelled_by(node)` for the group; each row uses its option label |
| `slider` | default, hover, pressed, focused, disabled | Arrow keys, PageUp, PageDown, Home, End | `Slider` | "Slider" |

Tab and Shift+Tab move focus between controls. A disabled control
leaves the focus order.

## Terminal notes

- Control sizes round to whole cells. A control border reserves a whole
  cell.
- The checkbox indicator paints as `[x]` or `[ ]` text.
- Pointer clicks work when the terminal reports SGR mouse events.
- Use `rows()` and `cols()` for gaps and padding around controls. A
  pixel gap below one cell rounds to zero. See [Units](../units.md).

## Example

The example builds one of each control. Run it with
`cargo run -p sigil-ui --example widget_controls`.

```rust,ignore
{{#include ../../../crates/sigil/examples/widget_controls.rs}}
```
