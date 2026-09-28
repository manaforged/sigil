# Overlays

Nodes that float above the tree — dialogs, menus, tooltips, toasts.
They mount in the top layer (`TopLayer`), so order them after the
content they cover, typically as the last child or the `modal` slot of
`app_shell`.

## Shared surface and placement

Every overlay uses one surface: the `overlay` fill, a 1 px
`overlay-border` border, the `OVERLAY_SHADOW` elevation, and the `md`
radius. Panels pad their rows by `Metrics::overlay_padding`.

Menu, dropdown, combobox, and command menu rows share one row layout:
the label, an optional shortcut hint, an optional chevron for a
submenu, and a check mark on the selected option. A disabled row uses
muted text and is never highlighted. A separator is inset to the row
padding.

Popups stay inside the window. A list popup opens below its trigger
and flips above it when the window has no room below. A tooltip opens
above its target and flips below it at the top edge. After the flip,
the popup is clamped to the window on both axes.

## Keyboard in lists

`menu`, `menu_bar` panels, `dropdown`, `combobox`, and `command_menu`
share one list behaviour:

- Arrow Up and Arrow Down move the highlight and skip disabled rows.
- Home and End jump to the first and last enabled row.
- A printable key jumps to the next row whose label starts with it
  (not in `combobox`, where keys go to the text field).
- Enter activates the highlighted row. On a closed dropdown or
  combobox, Enter or Arrow Down opens the list.
- Escape closes the list. A focus loss closes it too.
- A scrolling list keeps the highlighted row in view.

## modal and sheet

`modal(show: Signal<bool>) -> ModalBuilder` and
`sheet(show: Signal<bool>) -> ModalBuilder`. Both render a backdrop
plus a dialog; `sheet` is `modal(...).side(Side::Right)
.dismiss_on_backdrop(true)` — a right-edge drawer.

- `.title(impl Into<TextContent>)` — dialog heading; also announced
  when the dialog opens.
- `.child(node)` — dialog content.
- `.side(Side)` — pin to `Side::Left|Right|Bottom` instead of
  centering.
- `.trap(bool)` — focus trap inside the dialog (default true).
- `.escape_closes(bool)` — Escape closes the topmost open dialog
  (default true; a stack tracks multiple modals).
- `.dismiss_on_backdrop(bool)` — clicking the scrim writes `false`
  (default false for `modal`, true for `sheet`).
- `.footer(node)` — adds an action to the footer row. The footer sits
  under the content and aligns its actions to the right.
- `.actions(cancel, confirm)` — adds a cancel and confirm pair to the
  footer, in that order.

Visibility is display-driven from `show`; the node stays in the tree.

```rust,no_run
# use sigil::prelude::*;
# let _ = {
let open = signal(false);
// ... a button sets open.set(true)
modal(open)
    .title("Delete file")
    .dismiss_on_backdrop(true)
    .child(text("This cannot be undone.").nowrap())
    .actions(
        button("Cancel").neutral().activate(move || open.set(false)),
        button("Delete").destructive().activate(move || open.set(false)),
    );
# };
```

## menu and menu_bar

`menu(open: Signal<bool>) -> MenuBuilder` — a floating `Role::Menu`
panel. `.item(MenuItem)`, `.separator()`, `.label(name)` for
accessibility, `.anchor(x, y)` to pin a position (otherwise it follows
its layout slot). The panel is 192 px wide unless you set a width. It closes on Escape
and on a pointer-down outside the panel.

`MenuItem::new(label)` carries `.shortcut(hint)` (a displayed hint, not
a binding), `.disabled(bool)`, `.submenu(bool)` (shows the indicator —
you render the submenu yourself), and `.select(Fn())` which fires and
closes the menu.

`menu_bar()` builds the horizontal bar: `.menu(label, items)` appends a
named dropdown whose trigger toggles on click. The bar owns one
`open_idx` signal across its slots, so opening one menu replaces
another.

## command_menu

`command_menu() -> CommandMenuBuilder` — a nested, keyboard-driven menu
(`Role::Menu`, "Command Menu"), not a search palette. Build a tree of
entries:

```rust,no_run
# use sigil::prelude::*;
# let _ = {
command_menu()
    .item("New file", || { /* ... */ })
    .submenu("Export", |sub| {
        sub.item("PNG", || { /* ... */ })
            .item("SVG", || { /* ... */ })
    })
    .disabled("Print")
    .select(|label| tracing::info!(label, "ran"))
    .cancel(|| tracing::info!("cancelled"));
# };
```

Arrow Up/Down move the cursor, Enter or Arrow Right activates the item
or descends into a submenu, Arrow Left or Escape ascends — and calls
`.cancel` at the root. `.select` receives the activated label. For a
searchable Cmd-K palette, compose `text_field`, `virtual_list`, and a
`modal` — `command_menu` does not filter.

## tooltip

`tooltip(label) -> TooltipBuilder` then `.child(target)` wraps the
hovered node. The bubble uses the overlay surface and appears above
the target on hover. It sizes to its text up to 240 px wide, and longer
text wraps inside it.

- `.shortcut(Some("⌘K"))` — secondary hint text in the bubble.
- `.delay(ms)` — hover time before showing.
- `.clamp_top()` — open the bubble below the target instead of above
  it.

## toast and toast_queue

`toast(message: Signal<Option<String>>) -> NodeBuilder` — a single
bottom-edge status band, hidden while the signal is `None`. You own
the timeout.

`toast_queue(queue: ToastQueue) -> NodeBuilder` — a self-managing
stack. `ToastQueue::new()` is a `Copy` handle; `queue.info(msg)`,
`.success(msg)`, `.warning(msg)`, `.error(msg)` (or
`.push(ToastLevel, msg)`) append a `ToastEntry`. Entries expire after
`TOAST_TTL` (4 s) on a 250 ms sweep and at most `MAX_VISIBLE` (4) show
at once, newest at the bottom. The container is `Role::Status`, so
screen readers hear pushes.

## hint_overlay

`hint_overlay(&[HintEntry]) -> HintOverlayBuilder` where `HintEntry` is
`(chord, description)`. Renders a "Keyboard shortcuts" card of
`key — description` rows; `.entry(chord, desc)` appends more. It is
just a styled container — mount it inside `modal`/`sheet` or your own
top-layer node to make it a real overlay.

## States, keys, and accessibility

The visual catalog captures the closed and open state of each overlay,
and the hover and pressed rows of an open menu.

| Widget | States | Keys | Role | Name source |
| --- | --- | --- | --- | --- |
| `modal`, `sheet` | closed, open | Escape closes the top dialog; Tab stays inside while `.trap(true)` | `Dialog` | `.title()`, else "Dialog" |
| `menu` | open; rows hover and pressed | The list keys above | `Menu`, rows `MenuItem` | `.label()` |
| `command_menu` | default, hover, pressed, focused | Arrow keys, Enter, Escape | `Menu`, rows `MenuItem` | "Command Menu" |
| `tooltip` | hidden, shown on hover or keyboard focus | The bubble shows when the first focusable node in the target gets keyboard focus; blur or Escape hides it | `Tooltip` | The label, plus the shortcut in parentheses |
| `toast`, `toast_queue` | hidden, shown | none | `Status` | None; the message is the text child |
| `hint_overlay` | static | none | `GenericContainer` | None; the rows carry the text |

A modal announces its title when it opens. A toast queue is a live
`Status` region, so each push is announced.

## Terminal notes

- Overlays draw last, above clips. Shadows are omitted; a popup keeps
  its border.
- A tooltip needs pointer motion. Terminals that do not report motion
  never show it, so do not put required text only in a tooltip.
- A centred `modal` sizes to its content. `.width()` and `.height()`
  take pixels, not `cols()` or `rows()`; the host rounds them to whole
  cells.
- `toast_queue` sweeps expired entries every 250 ms, so the host wakes
  on that interval while a queue is mounted.

## Example

Run it with `cargo run -p sigil-ui --example widget_overlays`.

```rust,ignore
{{#include ../../../crates/sigil/examples/widget_overlays.rs}}
```
