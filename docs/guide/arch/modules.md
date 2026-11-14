# Modules

`crates/sigil` is the package `sigil-ui`, library `sigil`.

Downstream code depends on `sigil-ui`, not on `wgpu` or `winit` directly. The terminal host is the module `terminal` behind the feature `terminal`, which adds crossterm.

The public modules are `animation`, `app`, `clipboard`, `events`, `geometry`, `icons`, `layout`, `node`, `profile`, `reactive`, `scene`, `style`, `tasks`, `terminal`, `text`, and `widgets`. The others are internal and public only under the `kittest` feature, with `#[doc(hidden)]`.

`sigil::prelude` holds the items that most apps use: the containers, the widget functions, signals, tasks, tokens, and the event types. An item that is not in the prelude stays public in the module that owns it. For example, `Cx` and `NodeId` are in `sigil::node`, `TrackSize` is in `sigil::layout`, `SceneBuilder` is in `sigil::scene`, `Property` and `Animator` are in `sigil::animation`, and builder types such as `ButtonBuilder` are in `sigil::widgets`. The crate root holds `run`, `AppConfig`, `Host`, `View`, `exit`, `Error`, `ErrorKind`, and `AppResult`.

| Module | Owns |
| --- | --- |
| `hal` | Traits for the renderer, shell, text engine, accessibility, and scheduler. Layout constraint types (`Dim`, `Size`). |
| `reactive` | Signal graph, owner tree, `EventChannel`, phases. |
| `kernel` | The tree, builders, semantics, and reactive children. `sigil::node` is its public face: `Cx`, `NodeId`, `NodeBuilder`, `IntoNode`, `Role`, and the node functions. |
| `layout` | `LayoutEngine`, the container functions, `LayoutStyle`, `Length`, `Space`, the incremental drive. |
| `style` | `Paint`, `Token`, theme, metrics. |
| `events` | Hit testing, focus walk, pointer capture, handler builders, `DropEvent`, `DragPhase`, `ValueStep`. |
| `text` | Shaping, measurement, paragraph cache, fonts. The public items are `TextStyle`, `TextMeasure`, `measure_text`, caret and hit-test geometry, and the grapheme column helpers. |
| `scene` | `SceneBuilder`, `CommandScene`, and the drawing commands. A custom renderer reads commands such as `DrawPath` (`outline`, `fill`, `fill_rule`, `stroke`), `PathCommand`, `Gradient`, and `Stroke`. |
| `renderer` | wgpu, emit cache, layer pool. |
| `shell` | winit window and event loop. |
| `a11y` | AccessKit projection. |
| `tasks` | Tokio runtime, `task`, `spawn`, `Signal::writer`, `listen`, `Status`. |
| `animation` | Easing, keyframes, transitions, `Animator`, `Spring`, `every_frame`, and the runtime that plays them. |
| `app` | `run`, `Host`, frame loop, menu, tray, dialogs, clipboard, audio, pty. |
| `widgets` | The widget catalog, the `Widget` setter trait, and `Framed` and `WidgetFrame` for custom widgets. No window, no GPU. |
| `profile` | `SIGIL_PROFILE`, trace output, phase table. |

Modules are layered. `hal` and `reactive` sit at the bottom. `kernel`, `style`, `layout`, and `events` build on them. `widgets` and `renderer` sit above. `reactive` does not depend on the tree. The tree depends on signals, which is why `Signal` is `Copy` and lives in thread-local storage: the layout engine can read a style signal without owning a widget. The `terminal` module sits above everything and paints the same tree on a character grid.
