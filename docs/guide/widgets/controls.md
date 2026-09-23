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

```rust,ignore
let count = signal(0_i32);
button("Increment").primary().activate(move || {
    count.set(count.peek() + 1);
});
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
- `.pending(Signal<bool>)` — paints disabled and ignores activation
  while true; combines with `.disabled()` as a logical or.
- `.accessible(name)` — assistive name when the label is a glyph.

Activation fires on left-button release while the press began inside
and the pointer is still over, and on Enter or Space while focused. The
node is `Role::Button`, focusable, and carries a keyboard focus ring.

`icon_button(glyph, activate)` is `button(glyph).ghost().activate(f)`
— a frameless button whose label is a Unicode glyph. Chain
`.accessible("Play")` so the glyph gets an assistive name.

## checkbox

`checkbox(Signal<bool>) -> CheckboxBuilder`. A bordered box with a
vector check mark. `Role::CheckBox`; publishes `checked` to the
accessibility tree.

```rust,ignore
let agreed = signal(false);
checkbox(agreed).label("Accept terms").change(|on| {
    tracing::info!(on, "checkbox");
});
```

Key options: `.label(impl Into<TextContent>)` puts a label beside the
box and makes the row fill its width; `.tone(Tone)` — `Tone::Neutral`
checks in gray scale step 9, every other tone checks in `accent`;
`.change(Fn(bool))` is a side effect — the widget writes the signal
first, then calls the closure; `.disabled(Signal<bool>)`.

Pointer release, Enter, and Space flip the value. In the terminal host
the indicator degrades to `[x]` / `[ ]` text.

## toggle

`toggle(Signal<bool>) -> ToggleBuilder`. A sliding switch:
`Role::Switch`, a 36×20 px track with a thumb. Same surface as
checkbox: `.label()`, `.disabled(Signal<bool>)`, `.change(Fn(bool))`,
and the same pointer/Enter/Space activation. The accessible name is the
label text or "Toggle".

## radio_group

`radio_group(Signal<String>) -> RadioGroupBuilder`. A vertical stack of
options bound to a string key.

```rust,ignore
let mode = signal(String::from("auto"));
radio_group(mode)
    .option("auto", "Automatic")
    .option("manual", "Manual")
    .tone(Tone::Primary);
```

`.option(key, label)` appends a row (`Role::RadioButton`, publishes
`checked`). Arrow Down/Up move the selection with wraparound; Home and
End jump to the first and last option. The group is `Role::RadioGroup`,
takes one focus slot, and draws the focus ring on the checked row.

## slider

`slider(Signal<f32>, RangeInclusive<f32>) -> SliderBuilder`. A drag
thumb on a filled track. `Role::Slider`; publishes the value, the
range, and the step so assistive tech can issue Increment/Decrement.

```rust,ignore
let volume = signal(50.0_f32);
slider(volume, 0.0..=100.0)
    .step(5.0)
    .change(|v| tracing::info!(v, "volume"));
```

Key options:

- `.step(f32)` — quantizes every write; default 1.0, non-positive or
  non-finite steps fall back to 1.0.
- `.change(Fn(f32))` — fires on drag release and on each key step, not
  continuously during the drag.
- `.orientation(Orientation::Vertical)` — default Horizontal.
- `.length(px)` — fixes the track extent; the default fills the parent.
- `.disabled(Signal<bool>)`.

Keys: Arrow Right/Up add one step, Arrow Left/Down subtract,
PageUp/PageDown move ten steps, Home/End jump to the range ends. Drag
captures the pointer, so a slide that leaves the track still updates.
