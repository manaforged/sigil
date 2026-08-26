# Sigil examples

Each example is a `cargo run -p <name>` binary. The recommended reading
order goes top to bottom — the user-facing examples come first, the
headless / internals reference comes last.

Use `sigil::prelude::*`. Event handlers are `.activate`, not
`.on_activate`. Form submit is `.submit`, not `.on_submit`. Hover
class strings live on `interactive(...)`, not on a plain container.

## Hello path

| Example | Concepts | When to read |
|---|---|---|
| [`hello-counter`](hello-counter/) | `signal`, `derived`, `button(...).activate`, reactive `text(...)`. | First. One signal, one handler, one derived label. |
| [`hello-host`](hello-host/) | Same tree on `sigil::Host`. No window. Prints `painted` and `surfaces`. | Embed path. `Runtime::scope`, then layout. Feature `desktop` off. |
| [`hello-tui`](hello-tui/) | Same tree as hello-counter on `sigil::run_tui`. | Same widgets and events; the host is a cell grid, not wgpu, not a PTY. |
| [`hello-events`](hello-events/) | Pointer hit-testing, `on_pointer_down`, keyboard focus with `focusable(true)`. | How events flow through the tree on `sigil::launch`. |
| [`hello-style`](hello-style/) | `.class("p-4 bg-accent rounded-md")`, `style_reactive`, `interactive()`. | Static utility classes and hover on `interactive()`. |
| [`linear-row`](linear-row/) | One `interactive().class("flex-row … hover:bg-accent-9")` issue row. | Styling sentence. No `LayoutStyle` on the row. |
| [`hello-tree`](hello-tree/) | Headless `Cx` build, no window — prints the node tree. | Debug tree shape without rendering. |
| [`hello-layout`](hello-layout/) | Headless layout pass, prints computed rectangles. | Verify layout without rendering. |

## Catalog and dogfood

| Example | Concepts | When to read |
|---|---|---|
| [`widget-playground`](widget-playground/) | Live catalog: `icon_button`, `button.pending`, `card`, `avatar`, `sidebar`, palette recipe, hover row. | After the hello path. One window for every public widget. |
| [`player`](player/) | `sidebar` + `avatar` + `card` + `transport` bound to `Player`. Pass a file to `play_path`. Drag calls `seek`. | Named chrome. Feature `audio`. |
| [`pty-pane`](pty-pane/) | `splitter` + `terminal(lines)` + `Session::spawn`. | PTY pane, not `run_tui`. Feature `pty`. |
| [`vertical-slice`](vertical-slice/) | Panel, button, checkbox, dropdown, slider, scroll, progress, text field. | Combining widgets into one window. |
| [`todo`](todo/) | `menu_bar().class(...)`, `FileDialog`, tray, `open`. | Full-app smoke: styled GPU chrome + CRUD. |
| [`scratchpad`](scratchpad/) | `text_field` + save / revert / dirty. `text_edit` is the multi-line pick. | Editor-shaped state. |
| [`password-generator`](password-generator/) | `slider`, `checkbox`, `button.disabled`, derived readout. | Form controls. |
| [`timer-stopwatch`](timer-stopwatch/) | `sigil::every`, lap list. | Frame-clock timer. |
| [`regex-tester`](regex-tester/) | Two `text_field`s, derived match list. | Derived `Result` without panicking. |
| [`log-tailer`](log-tailer/) | `virtual_list` + filter + `chip` levels + `auto_pin_to_bottom`. | Viewer-class list. |
| [`csv-viewer`](csv-viewer/) | `table(TableState)` over sortable columns. | Data table. |
| [`file-browser`](file-browser/) | `splitter` + `virtual_list` + `tabs`. | Multi-pane shell. |
| [`api-explorer`](api-explorer/) | Same shell, mock HTTP catalog. | Tabs routing. |
| [`kanban-board`](kanban-board/) | Three columns, `DragState<(usize, usize)>`, `icon_button`, `modal`. | Drag between columns. |
| [`chat-client`](chat-client/) | `sidebar` + `avatar` + `text_edit` compose + `virtual_list`. | Kitchen-sink chat. |
| [`workbench`](workbench/) | Desktop workspace: rail, work log, compose, diffs. | Dense desktop layout. |
| [`strays`](strays/) | `table()` over live system data on `run_tui`. | Keyboard-first terminal app. |

FRICTION.md files in those folders are historical dogfood logs. A
**Current tree** stamp at the top of each stale log names what has
since landed in the prelude. Do not re-open a closed item.

## 1:1 and internals

| Example | Concepts | When to read |
|---|---|---|
| [`data-table`](data-table/) | 1:1 of GPUI 0.2.2 `examples/data_table.rs`: 10k rows, 24 columns, `virtual_list`. | Side-by-side with `cargo run --example data_table -p gpui`. Profiler page: [`showcase/compare/data-table.html`](../showcase/compare/data-table.html). |
| [`access-log`](access-log/) | 1M HTTP lines, 60×8 live tape, same book in Sigil and GPUI. | `cargo run --release -p access-log --bin access-log-sigil`. |
| [`internals/hello-rect`](internals/hello-rect/) | Direct kernel + scene-command surface; pre-dates `sigil::launch`. | Only when working below the launch facade. |
