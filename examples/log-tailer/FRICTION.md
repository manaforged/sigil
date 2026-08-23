# log-tailer — friction log

## Current tree (do not re-open)

Closed:

- `virtual_list`, `SelectionMode`, `SelectionState`, `derived` are in
  `sigil::prelude::*`.
- `virtual_list` items may be a `Vec` (snapshot) or a `Signal` (live).
- `.auto_pin_to_bottom(true)` pins a live tail.
- `chip(label, Tone)` / `badge(label, Tone)` for level pills.
- Widget builders implement `WidgetLayoutExt` (`.width` / `.height` /
  `.grow`). They are not `NodeBuilder`; pass them to `.child()` via
  `IntoNode`.

Open:

- Visible selected-row paint on `virtual_list` is still a per-row
  `style_reactive` job unless you use `table()`.
- Binary crates: `unreachable_pub` vs `redundant_pub_crate`. Allow one
  at the crate root.

Historical pass: ~35–45 minutes to a green headless smoke, mostly
clippy. `Signal<T>: Copy` and `text(signal)` were the good surprises.
