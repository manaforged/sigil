//! Navigation widgets: `tabs`, `menu`, `tooltip`, `hint_overlay`.

use sigil::prelude::*;
use sigil::widgets::patterns::{self, HintOverlayBuilder};
use sigil::widgets::{MenuItem, menu, tabs, tooltip};

use crate::{category, column, row};

pub(crate) fn navigation_section() -> NodeBuilder {
    let body = column(vec![
        tabs_row(),
        menu_row(),
        tooltip_row(),
        hint_overlay_row(),
    ]);
    category("Navigation widgets", body)
}

fn tabs_row() -> NodeBuilder {
    let active = signal(0_usize);
    let labels = vec![
        String::from("Overview"),
        String::from("Details"),
        String::from("Logs"),
    ];
    let widget = tabs(active, labels)
        .label("Section tabs")
        .content_for_fn(0, || {
            text("Tab 0 — overview pane").class(classes!("text-sm"))
        })
        .content_for_fn(1, || {
            text("Tab 1 — details pane").class(classes!("text-sm"))
        })
        .content_for_fn(2, || text("Tab 2 — logs pane").class(classes!("text-sm")))
        .into_container();
    patterns::field("tabs (3 tabs, factory bodies)", widget)
}

fn menu_row() -> NodeBuilder {
    let open = signal(true);
    let widget = menu(open)
        .label("File menu")
        .item(MenuItem::new("New").shortcut("Ctrl+N"))
        .item(MenuItem::new("Open").shortcut("Ctrl+O"))
        .separator()
        .item(MenuItem::new("Export").submenu(true))
        .item(MenuItem::new("Quit").shortcut("Ctrl+Q"));
    patterns::field("menu (open)", widget)
}

fn tooltip_row() -> NodeBuilder {
    let widget = tooltip("Save the file (Ctrl+S)")
        .child(button("Save"))
        .shortcut(Some("Ctrl+S"));
    patterns::field("tooltip (hover the button)", widget)
}

fn hint_overlay_row() -> NodeBuilder {
    let overlay = HintOverlayBuilder::new(&[
        ("Ctrl+S", "Save"),
        ("Ctrl+Z", "Undo"),
        ("?", "Toggle this overlay"),
    ]);
    patterns::field(
        "hint_overlay (3 chords)",
        row(vec![overlay_to_node(overlay)]),
    )
}

fn overlay_to_node(overlay: HintOverlayBuilder) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column())
        .child(overlay)
}
