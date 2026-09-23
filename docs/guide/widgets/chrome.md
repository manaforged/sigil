# Chrome

App frame and layout structure: shells, panels, navigation, splitters,
scrolling regions, and the `patterns` helpers that compose them.

## app_shell

`app_shell(top, work, status, modal) -> NodeBuilder`. The standard
window skeleton: a 48 px top bar, a growing work area, a 28 px status
bar, and an optional `Some(node)` mounted as an absolute overlay on
top. `modal` takes `Option<impl IntoNode>` — pass `None::<NodeBuilder>`
when you have no overlay.

```rust,ignore
app_shell(menu_bar().menu("File", items), work_area, status_line, None::<NodeBuilder>)
```

## panel and card

`panel() -> PanelBuilder` — a titled surface: `.title(str)` adds a 40 px
title bar, `.padding(px)` (default 16), `.elevation(tier)` picks the
surface tier, `.child(node)`, `.width(px)`.

`card(child) -> NodeBuilder` — the lighter variant: a `rounded-md`
bordered `bg-surface` column with 12 px padding and 8 px gaps.

## sidebar

`sidebar(child) -> NodeBuilder` — a fixed `Role::Navigation` column in
`bg-surface-hover`, 220 px wide, full height, no shrink.
`sidebar_width(px, child)` sets the width. Put navigation content
inside; it does not scroll or collapse on its own.

## tabs

`tabs(active: Signal<usize>, labels: Vec<String>) -> TabsBuilder`.
The strip is a `Role::TabList`; each tab activates on click or Enter.

- `.line()` / `.pill()` — strip chrome; default is bare.
- `.label(name)` — assistive name for the list.
- `.disabled(&[usize])` — disable specific indices.
- `.justify_list(JustifyContent)` — strip alignment.
- `.content_for(index, node)` — mount every pane up front and toggle
  display; state survives tab switches.
- `.content_for_fn(index, Fn() -> impl IntoNode)` — build only the
  active pane and rebuild on switch; cheap for heavy panes, but pane
  state resets each visit.
- `.reorderable(true)` + `.order(Signal<Vec<usize>>)` — drag to
  reorder; the order signal holds the permutation.
- `.closeable(true)` + `.close(Fn(usize))` — per-tab close buttons;
  the handler owns removing the label.
- `.into_container()` — build as a container node.

## splitter

`splitter(SplitterOrientation::Horizontal | Vertical) -> SplitterBuilder`.
Horizontal splits left/right (draggable vertical handles), Vertical
splits top/bottom. Handles are 6 px.

```rust,ignore
splitter(SplitterOrientation::Horizontal)
    .child(nav_pane, 240.0)
    .pane(content_pane).min(200.0).grow(1.0)
    .commit();
```

- `.child(node, initial_px)` — simple pane.
- `.pane(node)` — returns a `PaneBuilder` with `.initial(px)`,
  `.min(px)`, `.max(px)`, `.grow(factor)`; finish with `.commit()`,
  another `.pane(...)`, or pass it straight to `cx.add`.
- `.min_size(index, px)` / `.max_size(index, px)` — clamp by index.
- `.sizes_bind(Signal<Vec<f32>>)` — observe or persist pane sizes.
- `.snap_points(handle_index, &[px])` — snap a handle to offsets within
  an 8 px threshold.

## scroll

`scroll(content: NodeBuilder, extent) -> ScrollBuilder` where `extent`
is `f32` or `Signal<f32>` — the total content height. `scroll_auto
(content)` measures the content itself (`size_reactive`), which is the
common case.

- `.height(px)` (default 240) / `.width(px)` / `.fill()` — size the
  viewport.
- `.scroll_bind(Signal<f32>)` — expose the scroll offset.
- `.scroll_to_end()` — start scrolled to the bottom.
- `.auto_pin_to_bottom(bool)` — hold position at the end while content
  grows (logs, chat).

Wheel scrolling and a drawn scrollbar thumb are built in; for large
homogeneous rows prefer `virtual_list`, which virtualizes row
construction too — `scroll` still builds every child.

## surface

`surface(key: u64) -> NodeBuilder`. A fill-sized node carrying a
`Surface` extension — a hole in the widget tree that the host maps to a
native surface (video, GPU content) by `key`. The renderer does not
paint it; the host owns positioning and lifecycle of the real surface.

## dividers and status bar

`divider_horizontal()` / `divider_vertical()` — 1 px rules in the theme
border color, not hittable. `status_bar(children)` — a horizontal
`bg-surface-hover` strip that space-between's its children; the bottom
band of `app_shell`.

## patterns

Small composites in `sigil::widgets::patterns` (except `kv_row`, which
lives in `sigil::widgets::composition`; all are also re-exported at
`sigil::widgets`):

- `field(label, control)` — label stacked over a control with a 4 px
  gap.
- `validated_field(label, TextFieldBuilder, error: Signal<Option<String>>)`
  — additionally wires `invalid_when` and shows the error text under
  the field while the signal is `Some`.
- `kv_row(label, value)` / `kv_grid(&[(key, value)])` — aligned
  label/value rows; grid values render monospace.
- `list_row(selected, title, detail)` — a 24 px row with an ellipsized
  title, optional right-aligned detail, and a `bg-gray-5` wash when
  `selected`.
- `list_row_with(selected, title, ListRowSlots { marker, status, columns })`
  — `list_row` with an optional leading marker glyph and fixed-width,
  right-aligned `(text, width_px)` columns.
- `section_label(title)` — a small muted heading for groups.
- `empty_state(title, body)` — centered `Role::Status` placeholder.
- `notice_band(body, tone)` — a full-width tinted status strip.
- `hint_bar(keys, right)` — left key hints and a right accent label.
- `avatar(initials)` — a 28 px accent circle; the initials become the
  accessible name.
- `hairline()` — a 1×16 px vertical separator for toolbars.
- `spacer()` — a `grow(1)` block that pushes neighbors apart.

None of these hold their own state — drive them with signals from your
app and rebuild when the inputs change.
