#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::widgets::{CommandMenuBuilder, HintOverlayBuilder, MenuBarBuilder, MenuBuilder};

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - overlays").size(640, 560), build)
}

pub(crate) fn modal_demo(open: Signal<bool>) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().height(268.0))
        .child(text("Page behind the modal").nowrap())
        .child(
            modal(open)
                .title("Confirm action")
                .child(text("This modal is mounted at the root.").nowrap())
                .actions(button("Cancel"), button("Confirm").primary()),
        )
}

pub(crate) fn sheet_demo(open: Signal<bool>) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().height(268.0))
        .child(text("Page behind the sheet").nowrap())
        .child(
            sheet(open)
                .title("Details")
                .child(text("A right-edge sheet.").nowrap()),
        )
}

pub(crate) fn menu_demo(open: Signal<bool>) -> MenuBuilder {
    menu(open)
        .label("Edit")
        .anchor(16.0, 16.0)
        .item(MenuItem::new("Undo").shortcut("Ctrl+Z").activate(|| {}))
        .item(MenuItem::new("Redo").disabled(true))
        .separator()
        .item(MenuItem::new("Share").submenu(true))
}

pub(crate) fn menu_bar_demo() -> MenuBarBuilder {
    menu_bar().fill().menu(
        "File",
        [
            MenuItem::new("New").activate(|| {}),
            MenuItem::new("Open").shortcut("Ctrl+O").activate(|| {}),
            MenuItem::new("Quit").shortcut("Ctrl+Q").activate(|| {}),
        ],
    )
}

pub(crate) fn command_menu_demo() -> CommandMenuBuilder {
    command_menu()
        .item("Refresh", || {})
        .item("Open settings", || {})
        .disabled("Publish")
        .submenu("Export", |sub| sub.item("PNG", || {}).item("SVG", || {}))
}

pub(crate) fn tooltip_demo() -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().padding_top(40.0))
        .child(
            tooltip("Saves the document")
                .delay(0)
                .child(button("Hover me").ghost()),
        )
}

pub(crate) fn toast_demo(shown: bool) -> NodeBuilder {
    let message = shown.then(|| String::from("Changes saved"));
    container()
        .layout(LayoutStyle::flex_column().fill_width().height(88.0))
        .child(toast(signal(message)))
}

pub(crate) fn toast_queue_demo(shown: bool) -> NodeBuilder {
    let queue = ToastQueue::new();
    if shown {
        queue.success("Saved");
        queue.error("Upload failed");
    }
    container()
        .layout(LayoutStyle::flex_column().fill_width().height(128.0))
        .child(toast_queue(queue))
}

pub(crate) fn hint_overlay_demo() -> HintOverlayBuilder {
    hint_overlay(&[("Ctrl+K", "Palette"), ("Esc", "Close")])
}

fn build() -> impl IntoNode {
    scroll(
        container()
            .role(Role::Main)
            .layout(LayoutStyle::flex_column().gap(rows(1.0)).padding(cols(2.0)))
            .child(menu_bar_demo())
            .child(
                container()
                    .layout(LayoutStyle::flex_row().gap(cols(1.0)))
                    .child(modal_demo(signal(false)))
                    .child(sheet_demo(signal(false)))
                    .child(menu_demo(signal(false)))
                    .child(tooltip_demo()),
            )
            .child(command_menu_demo())
            .child(hint_overlay_demo())
            .child(toast_demo(false))
            .child(toast_queue_demo(false)),
    )
    .fill()
}
