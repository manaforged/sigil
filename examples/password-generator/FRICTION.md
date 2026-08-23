# password-generator — friction log

## Current tree (do not re-open)

Closed:

- `derived` is in `sigil::prelude::*`. Import it from the prelude, not
  `sigil::reactive::derived`.
- `button.disabled(Signal<bool>)` and `button.pending(Signal<bool>)`.
- `icon_button(glyph, activate).accessible(name)` for a glyph button.
- `number_field(Signal<T>, Range)` for an exact integer.
- `Signal::peek()` vs `Signal::get()`: peek in handlers, get in
  `derived`. Documented in `docs/book/01-reactive-model.md`.
- `sigil::testing::with_cx`.

Open:

- `slider` is `f32`. Integer length still rounds.

Closed: `sigil::clipboard::copy` / `paste` is the OS clipboard.
`launch` installs it on text fields.

Historical pass (v2): ~25 minutes from open-repo to green gate. The
prelude already had `slider` / `checkbox` / `button`.
