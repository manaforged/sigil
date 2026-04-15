# DR-004 — Text: Parley, with owned IME and editor completion

**Status:** Accepted
**Date:** 2026-04-14

## Context

Text is the hardest subsystem in any desktop UI framework. A complete
text stack includes:

- Shaping (glyph selection, kerning, ligatures) — HarfRust
- Font enumeration and fallback — Fontique
- Glyph and font parsing — Skrifa
- Bidi, segmentation, line breaking — ICU4X
- Paragraph layout — our own, or Parley
- Caret, selection, hit testing
- Editor operations (cursor movement, delete, undo, clipboard)
- IME integration per OS (Windows TSF, macOS NSTextInputClient, Linux IBus/Fcitx)
- AccessKit text property export

Research findings (April 2026):

- **Parley** v0.8.0 (March 2026) aggregates HarfRust + Fontique + Skrifa
  + ICU4X behind a paragraph API and exports AccessKit text properties.
  This is exactly the shape we want — rolling our own above HarfRust
  directly means independently integrating four crates plus writing the
  caret/selection/hit-testing layer.
- Parley's README still states "alpha-quality." PlainEditor was redesigned
  in v0.3.0 (breaking).
- **IME is explicitly incomplete.** Xilem (where the AccessKit author
  contributes) still has "missing glyph rendering in intermediate IME
  states, though final kanji display correctly" per the April 2025 Rust
  GUI survey. Parley alone does not give us a working IME.
- No production desktop app runs Parley end-to-end today. Highest-profile
  uses are Blitz, Xilem, and Vello demos — all Linebender frameworks in
  development.
- GPUI's own text input example is ~700 lines and is widely cited as the
  hardest component in Zed's framework. This is the baseline for how much
  work editor + IME is.

## Decision

Depend on `parley` upstream. Wrap behind a `TextEngine` trait in
`ui-hal`. The `ui-text` crate provides the adapter plus the layers
Parley does not give us.

**Explicit owned scope** in `ui-text`:

- IME integration on all three desktop OSes (Windows TSF, macOS
  NSTextInputClient, Linux IBus/Fcitx) — including the intermediate-glyph
  rendering Xilem currently misses
- Composition range model and preedit rendering
- Caret model and selection geometry
- Editor operations: word/line movement, delete, undo/redo, clipboard
- Text input primitive behaviour: password masking, triple-click line
  select, double-click word select, drag-select
- AccessKit text-range events and text properties

`harfrust` is consumed transitively through `parley`, never named
directly.

## Consequences

- Text + IME + editor is a multi-month subsystem. Phase 3 is scoped
  accordingly.
- We ship the most complete Parley-based IME integration in the Rust
  ecosystem. This is an explicit differentiator.
- If Parley breaks API repeatedly (more than twice per quarter) or if
  upstream choices become incompatible with our IME work, we fork Parley
  into `ui-text` and own the paragraph layer. Kill criteria are specific
  because the cost of forking is high.

## Kill criteria

Fork `parley` into `ui-text` when, and only when:

1. Parley ships more than two breaking API changes per quarter sustained
   over two quarters, **or**
2. A correctness issue we file against Parley's paragraph layout (not
   IME, which is our scope) is not fixable upstream within one release
   cycle and blocks shipping, **or**
3. Our IME integration work demonstrates that Parley's paragraph API does
   not expose enough for intermediate-glyph preedit rendering and
   upstream will not accept the needed API changes.

## References

- [Parley](https://github.com/linebender/parley)
- [Parley releases](https://github.com/linebender/parley/releases)
- [AccessKit](https://github.com/AccessKit/accesskit)
- [2025 Rust GUI survey — boringcactus.com](https://www.boringcactus.com/2025/04/13/2025-survey-of-rust-gui-libraries.html)
