# Scratchpad — friction log

## Current tree (do not re-open)

Closed:

- `text_edit` is public and in the prelude. This example still uses
  `text_field` so Enter can fire Save via `.submit`.
- `derived` is in `sigil::prelude::*`.
- `virtual_list` is in the prelude.
- `.submit()` is the method. There is no `.on_submit`.
- `status_bar`, `divider_horizontal` / `divider_vertical`.
- `sigil::testing::with_cx` for headless tree tests.

Open:

- No `.autofocus()` / `.focus_on_mount()` on `text_field`.
- `.change()` on `text_field` is a side-effect hook. The
  `Signal<String>` is already two-way bound.

Historical pass: a one-field demo was ~10 minutes after hello-counter.
The original log lost time hunting a `pub(crate)` `text_edit` that is
no longer gated.
