#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - overlays").size(640, 560), build)
}

pub(crate) fn modal_demo(open: Signal<bool>) -> NodeBuilder {
    column()
        .fill_width()
        .height(268.0)
        .gap(Space::None)
        .child(text("Page behind the modal").nowrap())
        .child(
            modal(open)
                .title("Confirm action")
                .child(text("This modal is mounted at the root.").nowrap())
                .actions(button("Cancel"), button("Confirm").primary()),
        )
}

pub(crate) fn sheet_demo(open: Signal<bool>) -> NodeBuilder {
    column()
        .fill_width()
        .height(268.0)
        .gap(Space::None)
        .child(text("Page behind the sheet").nowrap())
        .child(
            modal(open)
                .side(Side::Right)
                .dismiss_on_backdrop(true)
                .title("Details")
                .child(text("A right-edge sheet.").nowrap()),
        )
}

pub(crate) fn menu_demo(open: Signal<bool>) -> impl IntoNode {
    menu(open)
        .accessible("Edit")
        .anchor(Point::new(16.0, 16.0))
        .item(MenuItem::new("Undo").hint("Ctrl+Z").activate(|| {}))
        .item(MenuItem::new("Redo").disabled(true))
        .separator()
        .item(MenuItem::new("Share").submenu(true))
}

pub(crate) fn menu_bar_demo() -> impl IntoNode {
    menu_bar().fill_width().menu(
        "File",
        [
            MenuItem::new("New").activate(|| {}),
            MenuItem::new("Open").hint("Ctrl+O").activate(|| {}),
            MenuItem::new("Quit").hint("Ctrl+Q").activate(|| {}),
        ],
    )
}

pub(crate) fn command_menu_demo(open: Signal<bool>) -> impl IntoNode {
    command_menu(open)
        .item(MenuItem::new("Refresh").activate(|| {}))
        .item(MenuItem::new("Open settings").activate(|| {}))
        .item(MenuItem::new("Publish").disabled(true))
        .submenu("Export", |sub| {
            sub.item(MenuItem::new("PNG").activate(|| {}))
                .item(MenuItem::new("SVG").activate(|| {}))
        })
}

pub(crate) fn tooltip_demo() -> NodeBuilder {
    column()
        .fill_width()
        .padding_top(Space::raw(40.0, "example layout"))
        .gap(Space::None)
        .child(
            tooltip("Saves the document")
                .delay(0)
                .child(button("Hover me").ghost()),
        )
}

pub(crate) fn toast_demo(shown: bool) -> NodeBuilder {
    let message = shown.then(|| String::from("Changes saved"));
    column()
        .fill_width()
        .height(88.0)
        .gap(Space::None)
        .child(toast(signal(message)))
}

pub(crate) fn toast_queue_demo(shown: bool) -> NodeBuilder {
    let queue = ToastQueue::new();
    if shown {
        queue.success("Saved");
        queue.error("Upload failed");
    }
    column()
        .fill_width()
        .height(128.0)
        .gap(Space::None)
        .child(toast_queue(queue))
}

pub(crate) fn hint_overlay_demo() -> impl IntoNode {
    hint_overlay(&[("Ctrl+K", "Palette"), ("Esc", "Close")])
}

fn build() -> impl IntoNode {
    scroll(
        column()
            .role(Role::Main)
            .gap(rows(1.0))
            .padding(cols(2.0))
            .child(menu_bar_demo())
            .child(
                row()
                    .gap(cols(1.0))
                    .align_items(AlignItems::Stretch)
                    .child(modal_demo(signal(false)))
                    .child(sheet_demo(signal(false)))
                    .child(menu_demo(signal(false)))
                    .child(tooltip_demo()),
            )
            .child(command_menu_demo(signal(false)))
            .child(hint_overlay_demo())
            .child(toast_demo(false))
            .child(toast_queue_demo(false)),
    )
    .fill()
}
