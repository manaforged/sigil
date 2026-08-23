# Regex tester — friction log

## Current tree (do not re-open)

Closed:

- `text_edit` and `TextEditBuilder` are in `sigil::prelude::*`.
- `text_edit` line rows rebuild through `children_reactive`.
- `text_field` / `text_edit` have inherent `.class()`.
- `virtual_list` accepts a `Signal<Vec<T>>` (or `virtual_list_bind`).
- `Runtime` is in the prelude. Tests can also use `sigil::testing::with_cx`.
- `field(label, control)` for a labeled row.

This example still uses two single-line `text_field`s because the
pattern is one line. That is a product choice, not a missing widget.

Open:

- The prelude re-exports `Result` as `sigil::Result<T>` (one type
  parameter). For a non-Sigil error, write `core::result::Result<T, E>`
  or `sigil::AppResult<T>`.
- This example surfaces regex errors in a sibling label. Fields can
  also use `.invalid_when(Signal<bool>)` for a red border.

Historical pass: ~40 minutes to green `cargo test`, most of it clippy
and a then-missing `.class()` on `text_field`.
