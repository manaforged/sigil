# file-browser — friction log

## Current tree (do not re-open)

Closed:

- `splitter`, `SplitterOrientation`, `virtual_list`, `SelectionMode`,
  `SelectionState`, `tabs`, `TabsBuilder`, `derived` are in
  `sigil::prelude::*`.
- `VirtualListBuilder::select(Fn(usize))`.
- `splitter().pane(node).initial(px)` and `.child_constrained`.
- `divider_horizontal` / `divider_vertical`, `panel().title()`.
- `font-mono` is a real utility class.

Open:

- Putting a `virtual_list` in a splitter pane still wants an explicit
  viewport height more often than authors expect.
- Rust 2024: `fn build(...) -> impl IntoNode + use<>` when the
  function borrows a slice. Prefer owned `Vec` if the `'static` bound
  on `.child()` fights you.

Historical pass: ~35 minutes to `cargo test` ok. Tabs `content_for`
and `derived` tab bodies were the good surprises.
