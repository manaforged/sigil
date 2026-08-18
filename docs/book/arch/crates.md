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
| `sigil-app` | `launch`, frame loop, menu, tray, DPI. |
| `sigil-profile` | `SIGIL_PROFILE`, Perfetto, phase table. |
| `sigil` (`sigil-ui`) | Facade + widget catalog + prelude. |

Not default members (still in the workspace): `ide-core`,
`engine_adapter`, bench-parity crates.

## Dependency direction

```text
sigil-ui
  → app, widgets (in-tree), kernel, layout, style, events, reactive, tasks
app
  → kernel, layout, renderer, shell, a11y, reactive, tasks, profile
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
