# Crate map

The package on crates.io is `sigil-ui`. The library name is `sigil`.
Apps write `use sigil::prelude::*;`.

Downstream crates depend on `sigil-hal` traits (and `Dim` / `Size`),
not on `wgpu` or `winit` directly.

| Crate | Owns |
| --- | --- |
| `sigil-hal` | Traits: renderer, shell, text engine, a11y, scheduler. Layout **constraints** (`Dim`, `Size`). |
| `sigil-reactive` | Signal graph, owner tree, `EventChannel`, phases. Hidden: `Entity`, `EntityCx`, `Task`. |
| `sigil-kernel` | `Cx`, `NodeId`, builders, semantics, reactive children. |
| `sigil-layout` | `LayoutEngine`, `LayoutStyle`, incremental drive. |
| `sigil-style` | `StyleAtom`, theme, `classes!`, struct / fluent style. |
| `sigil-style-macros` | `classes!` token table. |
| `sigil-events` | Hit-test, focus walk, pointer capture, handler builders. |
| `sigil-text` | Shape, measure, paragraph cache, fonts. |
| `sigil-renderer` | wgpu, `CommandScene`, emit cache, layer pool. |
| `sigil-shell` | winit window and event loop. |
| `sigil-a11y` | AccessKit projection. |
| `sigil-tasks` | Tokio runtime, `spawn`, `dispatch_to_owner`, `Status`. |
| `sigil-actions` | Typed actions + `#[derive(Action)]`. |
| `sigil-animation` / `animation_runtime` | Transitions; `config(id)` for presets. |
| `sigil-app` | `launch`, `Host`, frame loop, menu, tray, dialogs, clipboard, `audio`, `pty`. |
| `sigil-widgets` | Widget catalog. No window, no GPU. `terminal` / `transport` / `surface` compile without those services. |
| `sigil-tui` | Terminal host. Same tree, character grid. Not a PTY. |
| `sigil-profile` | `SIGIL_PROFILE`, Perfetto, phase table. |
| `sigil` (`sigil-ui`) | Facade + prelude. Re-exports the widget catalog. |

Not default members (still in the workspace): the bench-parity crates.

## Dependency direction

```text
sigil-ui
  → app, widgets, kernel, layout, style, events, reactive, tasks
  → tui (feature tui)
app
  → kernel, layout, renderer, shell, a11y, reactive, tasks, profile
widgets
  → kernel, layout, style, events, reactive (no wgpu, no winit)
tui
  → kernel, layout, style, events, reactive, widgets (no wgpu)
layout
  → kernel, style, hal, reactive
renderer
  → kernel, layout, style, hal
reactive
  → (no kernel, no wgpu)
hal
  → no_std traits
```

`sigil-reactive` does not depend on the tree. The tree depends on
signals. That is why `Signal` is `Copy` and lives in TLS: the
layout engine can read a style signal without owning widgets.

## Three consume modes

| Mode | Entry | Swap |
|---|---|---|
| Desktop app | `sigil::launch` | Pumps `Host`. Default HAL + OS services |
| Embed (game / engine) | `sigil::Host` | Your loop and GPU. `Host::scene` needs feature `render`. |
| TUI | `sigil::run_tui` | Cell raster only |

HAL traits (`Shell`, `Renderer`, `TextEngine`, `A11yBridge`,
`Scheduler`) are the frame-loop connectors. Clipboard, audio, and PTY
are installable services, not HAL traits. `surface(key)` is a layout
hole. GPU and TUI skip it. `Host::surfaces` returns `(key, rect)`.
The caller places a native view. `launch` does not create a child
window. See [The application host](../07-application.md).
