#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::widgets::{CommandMenuBuilder, HintOverlayBuilder, MenuBarBuilder, TooltipBuilder};

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - overlays").size(640, 560), build)
}

fn modal_demo() -> NodeBuilder {
    let open = signal(false);
    container()
        .child(
            button("Delete file")
                .destructive()
                .activate(move || open.set(true)),
        )
        .child(
            modal(open)
                .title("Delete file")
                .child(text("This cannot be undone.").nowrap())
                .actions(
                    button("Cancel").activate(move || open.set(false)),
                    button("Delete")
                        .destructive()
                        .activate(move || open.set(false)),
                ),
        )
}

fn sheet_demo() -> NodeBuilder {
    let open = signal(false);
    container()
        .child(button("Details").activate(move || open.set(true)))
        .child(
            sheet(open)
                .title("Details")
                .child(kv_grid(&[("Owner", "ada"), ("Size", "4 KB")])),
        )
}

fn menu_demo() -> NodeBuilder {
    let open = signal(false);
    container()
        .child(button("More").activate(move || open.set(true)))
        .child(
            menu(open)
                .label("More")
                .item(MenuItem::new("Rename").activate(|| {}))
                .item(MenuItem::new("Archive").disabled(true)),
        )
}

fn menu_bar_demo() -> MenuBarBuilder {
    menu_bar()
        .menu(
            "File",
            [
                MenuItem::new("Open").shortcut("Ctrl+O").activate(|| {}),
                MenuItem::new("Quit").shortcut("Ctrl+Q").activate(|| {}),
            ],
        )
        .menu(
            "Edit",
            [MenuItem::new("Undo").shortcut("Ctrl+Z").activate(|| {})],
        )
}

fn command_menu_demo() -> CommandMenuBuilder {
    command_menu()
        .item("New file", || {})
        .submenu("Export", |sub| sub.item("PNG", || {}).item("SVG", || {}))
        .disabled("Print")
}

fn tooltip_demo() -> TooltipBuilder {
    tooltip("Saves the file")
        .shortcut(Some("Ctrl+S"))
        .child(button("Save"))
}

fn toast_demo() -> NodeBuilder {
    let message = signal(None::<String>);
    container()
        .child(button("Notify").activate(move || message.set(Some(String::from("Saved")))))
        .child(toast(message))
}

fn toast_queue_demo() -> NodeBuilder {
    let toasts = ToastQueue::new();
    container()
        .child(button("Upload").activate(move || toasts.success("Upload complete")))
        .child(toast_queue(toasts))
}

fn hint_overlay_demo() -> HintOverlayBuilder {
    hint_overlay(&[("Ctrl+K", "Command menu"), ("Ctrl+S", "Save")]).entry("Esc", "Close")
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
                    .child(modal_demo())
                    .child(sheet_demo())
                    .child(menu_demo())
                    .child(tooltip_demo()),
            )
            .child(command_menu_demo())
            .child(hint_overlay_demo())
            .child(toast_demo())
            .child(toast_queue_demo()),
    )
    .fill()
}
