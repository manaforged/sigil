# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
The workspace publishes in lockstep: every `sigil-*` crate shares one version.

## [0.1.0-alpha.1] — Unreleased

First public alpha. The facade publishes as `sigil-ui` (the `sigil` name was
already taken on crates.io); its library is still named `sigil`, so
`use sigil::prelude::*;` remains the single import user code needs.

### Added

- Signal-graph reactivity (`signal`, `derived`, `effect`, `batch`) with a
  measured, budgeted write path.
- Owned incremental layout engine, benchmarked against a Taffy reference corpus.
- Slug vector text pipeline plus hinted raster glyph atlas; IME-ready text
  editing; font fallback with CJK/emoji routing.
- Three-layer style system: `classes!` compile-time-checked utility classes,
  a `Style` struct API, and `style_reactive` signal binding; 12-step design
  tokens; tone × variant theming with WCAG-aware foregrounds.
- Widget catalog: button, checkbox, chip, code_edit, command_menu, divider,
  dot, dropdown, floating_text, gauge, icon, menu, modal, number_field, panel,
  progress, radio_group, scroll, slider, splitter, table (headless state),
  tabs, text_edit, text_field, toggle, tooltip, turn_strip, virtual_list.
- End-to-end AccessKit accessibility and IME.
- `sigil::launch` one-call app entry with DPI-aware logical sizing.
- Headless testing (`sigil::testing`) and a golden-image visual harness.

### Removed

- GitHub Actions orchestration, in favor of direct verification on owned
  native builders.
