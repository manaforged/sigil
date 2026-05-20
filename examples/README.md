# Sigil examples

Each example is a `cargo run -p <name>` binary. The recommended reading
order goes top to bottom — the user-facing examples come first, the
headless / internals reference comes last.

| Example | LOC | Concepts | When to read |
|---|---|---|---|
| [`hello-counter`](hello-counter/) | 33 | `signal`, `derived`, `button(...).on_activate`, reactive `text(...)`. | First. Minimum viable reactive UI — one signal, one event handler, one derived label. |
| [`hello-events`](hello-events/) | 35 | Pointer hit-testing, `on_pointer_down`, keyboard focus traversal with `focusable(true)`. | Second. Shows how events flow through the tree on `sigil::launch`. |
| [`hello-style`](hello-style/) | 64 | `.class("p-4 bg-accent rounded-md")`, `style_reactive`, `on_pointer_move_capture`. | Third. Static utility classes and reactive `&[StyleAtom]` swaps. |
| [`hello-tree`](hello-tree/) | 19 | Headless `Cx` build, no window — prints the node tree. | Reach for it when debugging tree shape without rendering. |
| [`hello-layout`](hello-layout/) | 65 | Headless Taffy layout pass, prints computed rectangles. | Reach for it when verifying layout output without rendering. |
| [`vertical-slice`](vertical-slice/) | 442 | Widget catalog cross-section: panel, button, checkbox, dropdown, slider, scroll, progress, text field. | Reference for combining real widgets into a window. |
| [`internals/hello-rect`](internals/hello-rect/) | 163 | Direct kernel + scene-command surface; pre-dates `sigil::launch`. | Advanced — read only when working below the launch facade. |
