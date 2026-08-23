# Chat-client dogfood — friction log

## Current tree (do not re-open)

Closed:

- `text_edit`, `derived`, `splitter`, `virtual_list`, `SelectionMode`,
  `Runtime` are in `sigil::prelude::*`.
- `field`, `patterns::app_shell`, `hint_overlay`.
- `NodeBuilder::children_reactive` — `text_edit` line rows rebuild.
- `virtual_list.auto_pin_to_bottom`.
- `TabsBuilder::into_container`.
- `splitter.child_constrained` / `.pane()`.
- `avatar`, `card`, `sidebar` / `sidebar_width`.
- `button.pending(Signal<bool>)`.
- Message rows in this example use `avatar`.

Open:

- This example's Send button is not wired to `.pending`.
- No `#[derive(SigilState)]` macro. Copy a `Copy` signal carrier by hand.

Historical pass (v2, ~1h 20min after kanban existed): `virtual_list` +
`splitter` + `tabs` + `text_field` + `text_edit` + `modal`. The
bottleneck was layout (which combinator makes the splitter fill), not
"is this widget in the prelude."

## What still reads as surprising

- `.label(...)` on `tabs` is the ARIA name; `label(text)` is a widget.
- `dismiss_on_backdrop(false)` and `escape_closes(true)` are both
  defaults. Backdrop click does not dismiss unless you opt in.
- Ctrl+Enter submits `text_edit`. Put that in `.placeholder` or a
  `tooltip` / `hint_overlay`. The placeholder string does not teach
  the key by itself.

## What worked

`Signal<T>: Copy` on a carrier struct. `launch((title, w, h), tree)`.
`with_cx` for headless tests. Modal Escape stack. `virtual_list`
row closure `(idx, &T)`. Utility classes for the 80% case.
