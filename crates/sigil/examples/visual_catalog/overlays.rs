use sigil::prelude::*;

use super::spec::{Build, State, node};

pub(crate) const BUILDERS: &[(&str, Build)] = &[
    ("menu", menu_row),
    ("modal", modal_row),
    ("sheet", sheet_row),
    ("tooltip", tooltip_row),
    ("toast", toast_row),
    ("toast_queue", toast_queue_row),
    ("command_menu", command_menu_row),
    ("hint_overlay", hint_overlay_row),
];

fn shown(state: State) -> Signal<bool> {
    signal(state != State::Default)
}

fn menu_row(state: State) -> NodeBuilder {
    node(
        menu(shown(state))
            .label("Edit")
            .anchor(16.0, 16.0)
            .item(MenuItem::new("Undo").shortcut("Ctrl+Z").activate(|| {}))
            .item(MenuItem::new("Redo").disabled(true))
            .separator()
            .item(MenuItem::new("Share").submenu(true)),
    )
}

fn modal_row(state: State) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().height(268.0))
        .child(text("Page behind the modal").nowrap())
        .child(
            modal(shown(state))
                .title("Confirm action")
                .child(text("This modal is mounted at the root.").nowrap())
                .actions(button("Cancel"), button("Confirm").primary()),
        )
}

fn sheet_row(state: State) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().height(268.0))
        .child(text("Page behind the sheet").nowrap())
        .child(
            sheet(shown(state))
                .title("Details")
                .child(text("A right-edge sheet.").nowrap()),
        )
}

fn tooltip_row(_state: State) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().padding_top(40.0))
        .child(
            tooltip("Saves the document")
                .delay(0)
                .child(button("Hover me").ghost()),
        )
}

fn toast_row(state: State) -> NodeBuilder {
    let message = (state == State::Shown).then(|| String::from("Changes saved"));
    container()
        .layout(LayoutStyle::flex_column().fill_width().height(88.0))
        .child(toast(signal(message)))
}

fn toast_queue_row(state: State) -> NodeBuilder {
    let queue = ToastQueue::new();
    if state == State::Shown {
        queue.success("Saved");
        queue.error("Upload failed");
    }
    container()
        .layout(LayoutStyle::flex_column().fill_width().height(128.0))
        .child(toast_queue(queue))
}

fn command_menu_row(_state: State) -> NodeBuilder {
    node(
        command_menu()
            .item("Refresh", || {})
            .item("Open settings", || {})
            .disabled("Publish")
            .submenu("Export", |sub| sub.item("PNG", || {}).item("SVG", || {})),
    )
}

fn hint_overlay_row(_state: State) -> NodeBuilder {
    node(hint_overlay(&[("Ctrl+K", "Palette"), ("Esc", "Close")]))
}
