# DR-027 - Text input contract

**Status:** Accepted
**Date:** 2026-04-17
**Depends on:** DR-025 (text-in-scene interim), DR-026 (widget substrate contract)

## Context

Widget Pack 2 adds `text_field`, the first first-party widget whose
primary state is user-authored text instead of a boolean or numeric
value. The existing widget substrate had already covered hover, press,
focus, disabled styling, pointer activation, keyboard activation, and
dragging, but text input asked for three extra decisions:

1. Whether `InteractionState` needs a content-widget extension.
2. Where the byte-indexed text editing state machine lives.
3. How focused widgets receive printable text that is not known at
   build time, including non-ASCII characters.

## Decision

`InteractionState` needs **no public API delta** for the text-field MVP.
The text field composes with the existing contract by using focus,
disabled state, hover state, and `FocusRing`, while handling editing
keys itself instead of using the button-style `activate_key` helper.

The editing state machine lives in `crates/sigil/src/widgets/text_edit.rs`
as `pub(crate) TextEdit`. It owns a `String` buffer and a byte-indexed
caret that is always snapped to a UTF-8 character boundary. It remains
internal until selection, clipboard, undo, or IME work proves a public
editing crate is needed.

Two adjacent substrate deltas ship with the MVP:

- `sigil-core::A11yProps` can carry `text_value: Signal<String>` and
  `placeholder: String` through `NodeBuilder::aria_text_value_signal`
  and `NodeBuilder::aria_placeholder`.
- `sigil-events::NodeEventsExt::on_key_any` lets a focused widget
  receive arbitrary key-down events after registered shortcut handlers.
  `text_field` uses this only for `KeyEvent::text` insertion; Backspace,
  Delete, arrows, Home, End, and Enter remain explicit key handlers.

The public widget API is:

```rust
pub fn text_field(value: Signal<String>) -> TextFieldBuilder;

impl TextFieldBuilder {
    pub fn placeholder(self, text: impl Into<TextContent>) -> Self;
    pub const fn disabled(self, flag: Signal<bool>) -> Self;
    pub fn on_change(self, f: impl Fn(&str) + 'static) -> Self;
    pub fn on_submit(self, f: impl Fn(&str) + 'static) -> Self;
}
```

## Rationale

**Why no `InteractionState` delta.** Text fields need focus and disabled
state, but they do not need press-to-activate semantics. Reusing
`InteractionState::install` gives the widget focus-ring and pointer
state consistency without teaching the substrate about carets.

**Why `TextEdit` is crate-private.** The MVP has no selection,
clipboard, undo, multiline navigation, or IME composition. Exporting an
editing model before those constraints are known would freeze the wrong
surface.

**Why `on_key_any` is needed.** The existing key API dispatches only
registered shortcuts. A text field cannot pre-register every possible
printable character. `on_key_any` is the smallest event-router addition
that lets content widgets consume `KeyEvent::text` while preserving
specific shortcut handlers for named keys.

**Why text value and placeholder live in `sigil-core`.** Accessibility
metadata is attached to the node model, not the widget implementation.
Sliders already carry a numeric value through `A11yProps`; text inputs
need the same first-class semantic path for a string value and
placeholder.

## Consequences

- `text_field` is a real single-line plaintext widget: typing,
  Backspace, Delete, arrows, Home, End, Enter submit, disabled handling,
  focus ring, placeholder paint, and text-input accessibility metadata.
- Content-oriented widgets can reuse `on_key_any` instead of registering
  large shortcut tables for printable input.
- `InteractionState` remains the shared visual/focus contract for
  built-in widgets, but activation remains opt-in at the widget layer.
- The vertical-slice example now includes a Notes field whose typed
  value flows into the status bar and whose submitted values render
  below the field.

## Deferred Scope

| Follow-up | Owner |
| --- | --- |
| IME composition and committed preedit rendering | Phase 14 |
| Selection model and cursor rendering | Pack 2b |
| Clipboard bridge | Pack 2c |
| Undo and redo edit log | Pack 2d |
| Multiline text area | Pack 3 |
| Password masking | Future widget pack |

## Kill Criteria

Supersede this DR when:

- Phase 14 lands IME composition and needs a richer text-input event
  contract.
- A selection model is proposed and `TextEdit` needs to expose anchors,
  ranges, or caret painting.
- AccessKit integration requires a different internal representation
  for string values, placeholders, or text selection.
- Multiple content widgets need more editor surface than `TextEdit`
  can provide as a crate-private helper.

## References

- `crates/sigil/src/widgets/text_field.rs`
- `crates/sigil/src/widgets/text_edit.rs`
- `crates/sigil-core/src/semantics.rs`
- `crates/sigil-events/src/handlers.rs`
- `examples/vertical-slice/src/main.rs`
- `docs/plans/widget-pack-2-text-field-mvp.md`
