# Modules

`crates/sigil` is the package `sigil-ui`, library `sigil`.

Downstream code depends on `sigil-ui`, not on `wgpu` or `winit`
directly. The terminal host is the module `terminal` behind the
feature `terminal`, which adds crossterm.

| Module | Owns |
| --- | --- |
| `hal` | Traits for the renderer, shell, text engine, accessibility, and scheduler. Layout constraint types (`Dim`, `Size`). |
| `reactive` | Signal graph, owner tree, `EventChannel`, phases. |
| `kernel` | `Cx`, `NodeId`, builders, semantics, reactive children. |
| `layout` | `LayoutEngine`, `LayoutStyle`, the incremental drive. |
| `style` | `StyleAtom`, theme, `parse_classes`, struct and fluent style. |
| `events` | Hit testing, focus walk, pointer capture, handler builders. |
| `text` | Shaping, measurement, paragraph cache, fonts. |
| `renderer` | wgpu, `CommandScene`, emit cache, layer pool. |
| `shell` | winit window and event loop. |
| `a11y` | AccessKit projection. |
| `tasks` | Tokio runtime, `task`, `spawn`, `signal_writer`, `on_message`, `Status`. |
| `animation` and `animation_runtime` | Transitions; `config(id)` for presets. |
| `app` | `run`, `Host`, frame loop, menu, tray, dialogs, clipboard, audio, pty. |
| `widgets` | The widget catalog. No window, no GPU. |
| `profile` | `SIGIL_PROFILE`, trace output, phase table. |

Modules are layered. `hal` and `reactive` sit at the bottom. `kernel`,
`style`, `layout`, and `events` build on them. `widgets` and `renderer`
sit above. `reactive` does not depend on the tree. The tree depends on
signals, which is why `Signal` is `Copy` and lives in thread-local
storage: the layout engine can read a style signal without owning a
widget. The `terminal` module sits above everything and paints the same
tree on a character grid.
