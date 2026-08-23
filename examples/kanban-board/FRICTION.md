# Kanban Board — friction log

## Current tree (do not re-open)

Closed:

- `.submit()` is the form-submit method. There is no `.on_submit`.
- `text_edit` is public and in the prelude. `text_field` is single-line.
- `splitter`, `SplitterOrientation`, `virtual_list`, `SelectionMode`
  are in `sigil::prelude::*`.
- `icon_button`, `dropdown`, `toast`, `chip` / `badge`, `field`,
  `progress`, `button.pending`.
- This example is three columns with `DragState<(usize, usize)>` and
  `move_card`. Edit uses `icon_button`.

Open:

- No first-party context menu on a card. Compose `menu(Signal<bool>)`
  or the native bar.
- Binary crates still hit `unreachable_pub` vs `redundant_pub_crate`.
  Allow one at the crate root.

Historical pass (2026-05-20): ~3.5 hours to first compiling build.
What worked then still works: `Signal` is `Copy`, `tabs().content_for`,
`modal` Escape, `BoardState` as a flat carrier.
