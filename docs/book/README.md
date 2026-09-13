# The Sigil Book

How to write native Rust UIs with Sigil, and how the framework is built.

The widget catalog ships in the crate. You do not add a second
component package.

```text
mdbook serve docs/book
```

The server opens the book at `http://localhost:3000`. Build a static
site with `mdbook build docs/book`. The output goes to
`docs/book/book/` and is not committed.

Companion material: the `sigil-examples` checkout (compile-checked apps,
including `workbench`) and the rustdoc on `sigil-gui`.

## Guide

- [Quickstart](quickstart.md) — install, first app, three hosts
- [Widgets](widgets.md) — the widget set
- [Terminal host](terminal.md) — the character-grid host

## Architecture

- [Overview](arch/overview.md) — what happens in one frame
- [State](arch/state.md) — signals, owners, async tasks
- [Painting](arch/painting.md) — tree, layout, text, renderer
- [Platform](arch/platform.md) — shell, input, accessibility
- [Modules](arch/modules.md) — the module map
