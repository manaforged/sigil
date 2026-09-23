# Overlays

Nodes that float above the tree — dialogs, menus, tooltips, toasts.
They mount in the top layer (`TopLayer`), so order them after the
content they cover, typically as the last child or the `modal` slot of
`app_shell`.

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

Visibility is display-driven from `show`; the node stays in the tree.

```rust,ignore
let open = signal(false);
// ... a button sets open.set(true)
modal(open)
    .title("Delete file")
    .dismiss_on_backdrop(true)
    .child(label("This cannot be undone."))
    .child(button("Delete").destructive().activate(move || open.set(false)));
```

## menu and menu_bar

`menu(open: Signal<bool>) -> MenuBuilder` — a floating `Role::Menu`
panel. `.item(MenuItem)`, `.separator()`, `.label(name)` for
accessibility, `.anchor(x, y)` to pin a position (otherwise it follows
its layout slot). The panel closes on Escape; with `.anchor` set, a
pointer-down outside the panel also writes `open.set(false)`.

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

```rust,ignore
command_menu()
    .item("New file", || { /* ... */ })
    .submenu("Export", |sub| {
        sub.item("PNG", || { /* ... */ })
            .item("SVG", || { /* ... */ })
    })
    .disabled("Print")
    .select(|label| tracing::info!(label, "ran"))
    .cancel(|| tracing::info!("cancelled"));
```

Arrow Up/Down move the cursor, Enter or Arrow Right activates the item
or descends into a submenu, Arrow Left or Escape ascends — and calls
`.cancel` at the root. `.select` receives the activated label. For a
searchable Cmd-K palette, compose `text_field`, `virtual_list`, and a
`modal` — `command_menu` does not filter.

## tooltip

`tooltip(label) -> TooltipBuilder` then `.child(target)` wraps the
hovered node. The bubble (accent background, ~160×28 px) appears above
the target on hover.

- `.shortcut(Some("⌘K"))` — secondary hint text in the bubble.
- `.delay(ms)` — hover time before showing.
- `.clamp_top()` — keep the bubble at the target's top edge instead of
  floating above it (for edge-of-window targets).

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
