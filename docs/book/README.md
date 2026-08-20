# The Sigil Book

How to write native Rust UIs with Sigil, and how the framework is built.
The widget catalog (buttons, fields, tables, lists, charts, menus)
lives in the crate. You do not add a second component package.

The **Guide** is the author path: first window, the box, signals, style,
widgets, tests, profiling.

**Architecture** is the system path: crate map, why state is a signal
graph, the owner tree, the frame loop, layout, the command scene, async,
and accessibility.

```text
mdbook serve docs/book
```

Opens the book at `http://localhost:3000`. Build a static site with
`mdbook build docs/book` (output is `docs/book/book/`, gitignored).

Companion material: `examples/` (compile-checked apps, including
`shell-demo`) and rustdoc on `sigil-ui`.

## Guide

- [Quickstart](00-quickstart.md) — install, first app, four moving parts
- [The box](01-layout.md) — one `LayoutStyle`
- [The reactive model](01-reactive-model.md) — signals, derived, spawn
- [Styling](02-styling.md) — classes, cascade, theme
- [Widgets](03-widgets.md) — catalog
- [Testing](04-testing.md) — headless harnesses
- [Profiling](05-profile.md) — `SIGIL_PROFILE`, Perfetto

## Architecture

- [Overview](arch/overview.md) — data flow from `launch` to GPU
- [Signals, not entities](arch/signals.md) — the state model
- [Owner tree](arch/owners.md) — dispose and cleanup
- [Frame loop](arch/frame.md) — Effect → Layout → Paint
- [Node tree](arch/tree.md) — `Cx`, identity, keyed children
- [Layout engine](arch/layout.md) — owned incremental boxes
- [Renderer](arch/renderer.md) — `CommandScene`, emit cache
- [Async](arch/async.md) — `spawn` → `Signal<Status>`
- [Accessibility](arch/a11y.md) — AccessKit on paint
- [Crate map](arch/crates.md) — what each crate owns
