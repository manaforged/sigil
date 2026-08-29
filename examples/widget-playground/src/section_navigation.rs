//! Navigation widgets: `tabs`, `menu_bar`, `menu`, `tooltip`, `hint_overlay`.

use sigil::prelude::*;
use sigil::widgets::patterns::{self, HintOverlayBuilder};
use sigil::widgets::{MenuItem, menu, menu_bar, tabs, tooltip};

use crate::{category, column, row};

pub(crate) fn navigation_section() -> NodeBuilder {
    let body = column(vec![
        tabs_row(),
        menu_row(),
        tooltip_row(),
        hint_overlay_row(),
        issue_row(),
        palette_row(),
        tree_row(),
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
    let popup = menu(open)
        .label("File menu")
        .item(MenuItem::new("New").shortcut("Ctrl+N"))
        .item(MenuItem::new("Open").shortcut("Ctrl+O"))
        .separator()
        .item(MenuItem::new("Export").submenu(true))
        .item(MenuItem::new("Quit").shortcut("Ctrl+Q"));
    let bar = menu_bar()
        .class("px-2")
        .menu("File", [MenuItem::new("New"), MenuItem::new("Open")])
        .menu("Edit", [MenuItem::new("Undo")]);
    column(vec![
        patterns::field("menu_bar (styled GPU bar)", bar),
        patterns::field("menu (open popup)", popup),
    ])
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

fn issue_row() -> NodeBuilder {
    let row = interactive(container().child(text("SIG-12  Hover this row").class("text-sm")))
        .class("p-2 rounded-md bg-surface hover:bg-accent-9");
    patterns::field("issue row (hover class)", row)
}

fn palette_row() -> NodeBuilder {
    let query = signal(String::new());
    let hits = derived(move || {
        let q = query.get();
        ["New issue", "Assign", "Cycle", "Copy link"]
            .into_iter()
            .filter(|row| row.to_lowercase().contains(&q.to_lowercase()))
            .map(str::to_string)
            .collect::<Vec<_>>()
    });
    let list = virtual_list(hits, |_i, row| {
        crate::wrap(text(row.clone()).class(classes!("text-sm")))
    })
    .row_height(22.0)
    .viewport_height(96.0);
    let body = container()
        .layout(LayoutStyle::flex_column().gap(8.0))
        .child(text_field(query).placeholder("Filter…"))
        .child(list);
    patterns::field("palette (text_field + virtual_list)", body)
}

fn tree_row() -> NodeBuilder {
    let rows = vec![
        (0_u32, "Engineering"),
        (1, "Platform"),
        (1, "App"),
        (0, "Design"),
    ];
    let list = virtual_list(rows, |_, (depth, label)| {
        #[allow(clippy::cast_precision_loss, reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale")]
        let pad = *depth as f32 * 16.0;
        crate::wrap(
            container()
                .layout(LayoutStyle::flex_row().padding_left(pad))
                .child(text(*label).class(classes!("text-sm"))),
        )
    })
    .row_height(22.0)
    .viewport_height(96.0);
    patterns::field("indented list (Teams)", list)
}
