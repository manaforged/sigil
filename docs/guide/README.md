# The Sigil Book

How to write native Rust UIs with Sigil, and how the framework is built.

```text
mdbook serve docs
```

The server opens the book at `http://localhost:3000`. Build a static site with `mdbook build docs`. The output goes to `docs/book/` and is not committed.

The [examples index](../reference/sigil-ui/examples/index.md) lists the runnable examples used in this guide.

## Guide

- [Quickstart](quickstart.md): install, first app, three hosts
- [Widgets](widgets.md): the widget set
- [GUI and TUI](hosts.md): what each host owns, and how to split host code
- [Units](units.md): the two kinds of layout length
- [Recipes](recipes.md): short answers to common jobs
- [Terminal host](terminal.md): the character-grid host
- [Web host](web.md): the browser host
- [Hot reload](hot-reload.md): patch a running app when you save a file
- [Testing](testing.md): the harness, snapshots, and goldens
- [Headless driving and automation](automation.md): drive an app from a script
- [Signing](signing.md): sign an update package
- [Packing](packing.md): build a macOS or Windows app bundle
- [Troubleshooting](troubleshooting.md): no GPU, images in tmux, and terminal color
- [API map](../api.md): each job mapped to its symbol

## Architecture

- [Overview](arch/overview.md): what happens in one frame
- [State](arch/state.md): signals, owners, async tasks
- [Painting](arch/painting.md): tree, layout, text, renderer
- [Platform](arch/platform.md): shell, input, accessibility
- [Modules](arch/modules.md): the module map

## Decisions

- [Decision records](decisions/README.md): accepted architectural decisions
