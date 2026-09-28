# Chrome

App frame and layout structure: shells, panels, navigation, splitters,
scrolling regions, and the `patterns` helpers that compose them.

## app_shell

`app_shell(top, work, status, modal) -> NodeBuilder`. The standard
window skeleton: a 48 px top bar, a growing work area, a 28 px status
bar, and an optional `Some(node)` mounted as an absolute overlay on
top. The top and status bands use the `surface-subtle` chrome fill with
a 1 px `border` rule toward the work area, an 8 px side inset, and
vertically centered content. The work area uses `surface`. `modal` takes `Option<impl IntoNode>` — pass `None::<NodeBuilder>`
when you have no overlay.

```rust,no_run
# use sigil::prelude::*;
# let items = vec![MenuItem::new("Quit")];
# let work_area = container();
# let status_line = container();
# let _ = {
app_shell(menu_bar().menu("File", items), work_area, status_line, None::<NodeBuilder>)
# };
```

`menu_bar()` draws the same chrome: a 32 px `surface-subtle` bar with a
1 px bottom border. Each trigger is a centered 22 px item with a small
radius. It shows `surface-hover` on hover, `surface-pressed` while
pressed, and keeps `surface-pressed` while its menu is open.

## panel and card

`panel() -> PanelBuilder` — a titled surface: `.title(str)` adds a
medium-weight title 8 px above the body, `.padding(px)` (default 16),
`.elevation(tier)` picks the surface tier, `.child(node)`, `.width(px)`.
Tier 0 is a flat `surface` fill. Tier 1 and above add a 1 px `border`.

`card(child) -> NodeBuilder` — the lighter variant: a `rounded-md`
`surface` column with the same 1 px `border`, 12 px padding, and 8 px
gaps. A panel at tier 1 and a card share one fill and one border.

## sidebar

`sidebar(child) -> NodeBuilder` — a fixed `Role::Navigation` column in
the `surface-subtle` chrome fill with a 1 px right border, 220 px wide,
full height, no shrink. Content sits inside an 8 px inset with 4 px
gaps.
`sidebar_width(px, child)` sets the width. Put navigation content
inside; it does not scroll or collapse on its own.

## tabs

`tabs(active: Signal<usize>, labels: Vec<String>) -> TabsBuilder`.
The strip is a `Role::TabList`; each tab activates on click or Enter.
The active tab label is semibold in the surface text colour; other
labels are medium weight in gray step 11. The bare strip also gives
the active tab a pressed surface, and the line strip an accent
underline. The pane sits 8 px (`space(2)`) below the strip.

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
splits top/bottom. Each handle is a 6 px hit area that draws a 1 px
`border` rule in its center. The area fills with `surface-hover` on
hover and `surface-pressed` while dragged.

```rust,no_run
# use sigil::prelude::*;
# let nav_pane = container();
# let content_pane = container();
# let _ = {
splitter(SplitterOrientation::Horizontal)
    .child(nav_pane, 240.0)
    .pane(content_pane).min(200.0).grow(1.0)
    .commit();
# };
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

Wheel scrolling and a drawn scrollbar are built in. The track is 8 px
wide and as tall as the viewport. The fully rounded thumb is 4 px wide
at rest. It widens to 8 px and the track shows `surface-subtle` while
the pointer is over the track or the thumb is dragged. For large
homogeneous rows prefer `virtual_list`, which virtualizes row
construction too — `scroll` still builds every child.

## surface

`surface(key: u64) -> NodeBuilder`. A fill-sized node carrying a
`Surface` extension — a hole in the widget tree that the host maps to a
native surface (video, GPU content) by `key`. The renderer does not
paint it; the host owns positioning and lifecycle of the real surface.

## dividers and status bar

`divider_horizontal()` / `divider_vertical()` — 1 px rules in the theme
border color, not hittable. `status_bar(children)` — a 28 px
`Role::Status` strip in the `surface-subtle` chrome fill with a 1 px top
border and an 8 px side inset. Every child except the last packs to the
left; the last child aligns right. A single child stays on the left.

## patterns

Small composites in `sigil::widgets::patterns` (except `kv_row`, which
lives in `sigil::widgets::composition`; all are also re-exported at
`sigil::widgets`):

- `field(label, control)` — label stacked over a control with a 4 px
  gap.
- `validated_field(label, TextFieldBuilder, error: Signal<Option<String>>)`
  — additionally wires `invalid_when` and shows the error text under
  the field while the signal is `Some`.
- `kv_row(label, value)` — a row at least 28 px tall. The key is a
  muted 16-column cell that ellipsizes. The value starts 12 px after it
  and fills the rest, so values in a stack align.
- `kv_grid(&[(key, value)])` — a stack of `kv_row`s with a 1 px
  divider between rows; values render monospace.
- `list_row(selected, title, detail)` — a 24 px row with an ellipsized
  title and optional right-aligned detail. A selected row has the
  `accent-5` wash. Other rows show `surface-hover` on hover and
  `surface-pressed` while pressed.
- `list_row_with(selected, title, ListRowSlots { depth, disclosure, marker, status, columns })`
  — `list_row` with tree slots, an optional leading marker glyph, and
  fixed-width, right-aligned `(text, width_px)` columns. `depth` indents
  the row by two columns per level. `disclosure` is a two-column chevron
  slot: `Disclosure::Collapsed` draws a right chevron,
  `Disclosure::Expanded` a down chevron, `Disclosure::Leaf` an empty
  slot so leaves align with their siblings, and `Disclosure::None`
  omits the slot. Collapsed and expanded rows report their state to
  accessibility. Build a tree by emitting one `list_row_with` per
  visible node.
- `section_label(title)` — a small muted heading for groups.
- `empty_state(title, body)` — centered `Role::Status` placeholder.
- `notice_band(body, tone)` — a full-width tinted status strip.
- `hint_bar(keys, right)` — left key hints and a right accent label.
- `avatar(initials)` — a 28 px accent circle; the initials become the
  accessible name.
- `hint_overlay(hints)` — a popup-surface list of shortcut rows with
  8 px gaps.
- `hairline()` — a 1×16 px vertical separator for toolbars.
- `spacer()` — a `grow(1)` block that pushes neighbors apart.

None of these hold their own state — drive them with signals from your
app and rebuild when the inputs change.

## States, keys, and accessibility

The visual catalog captures these states. The frame widgets (`panel`,
`card`, `surface`, `sidebar`, `status_bar`, `app_shell`) are static.

| Widget | States | Keys | Role | Name source |
| --- | --- | --- | --- | --- |
| `menu_bar` | default, hover, pressed, open | Enter or Space on a trigger; the open panel uses the list keys in [Overlays](overlays.md#keyboard-in-lists) | Triggers are `Button` | The trigger label |
| `tabs` | default, hover, pressed, focused, disabled, selected | Tab enters the active tab only; Left, Right, Home, and End move focus between tabs and select the tab; Enter activates | `TabList`, tabs `Tab` | `.label()` for the list, else "Tabs"; each tab uses its label |
| `splitter` | default, hover, pressed, focused | Arrow keys move the handle by 16 px; Home and End move it to the minimum and maximum | Handles are `Splitter` with a value: the size of the pane before the handle | "Resize panes" |
| `scroll`, `scroll_auto` | default, hover | Wheel, scrollbar drag; when focused, arrow keys, Page Up, Page Down, Home, and End. The area is a Tab stop only when its content overflows | `ScrollView` | None |
| `sidebar` | static | none | `Navigation` | None |
| `status_bar` | static | none | `Status` | None; the children carry the text |

Arrow keys do not move between tabs, and a splitter handle does not
take focus. Give a keyboard user another path, for example a
shortcut that sets the tab signal.

## Terminal notes

- `app_shell` bars are fixed pixel heights. In a terminal each bar
  takes its height rounded to whole cells, so the frame costs more rows
  than a hand-built frame. In a terminal-only app, build the frame from
  a `container()` with `rows()` heights.
- `sidebar` is 220 px wide. `sidebar_width(px, child)` takes pixels,
  not `cols()`. The host rounds the width to whole cells.
- `tabs` labels should fit on one row. Short labels avoid a wrap in a
  narrow terminal.
- A scroll viewport clips its content. The wheel scrolls three lines
  per notch.
- `surface` has no terminal form. The host does not paint it.

## Example

Run it with `cargo run -p sigil-ui --example widget_chrome`.

```rust,ignore
{{#include ../../../crates/sigil/examples/widget_chrome.rs}}
```
