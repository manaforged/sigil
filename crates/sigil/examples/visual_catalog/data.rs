use sigil::prelude::*;

use super::spec::{Build, State, node, off};

pub(crate) const BUILDERS: &[(&str, Build)] = &[
    ("table", table_row),
    ("virtual_list", virtual_list_row),
    ("virtual_list_fill", virtual_list_fill_row),
    ("list_row", list_row_row),
    ("list_row_tree", tree_rows),
    ("kv_grid", kv_grid_row),
    ("kv_row", kv_row_row),
    ("chart", chart_row),
    ("heatmap", heatmap_row),
    ("diff", diff_row),
];

const PATCH: &str = "--- a/src/app.rs\n+++ b/src/app.rs\n@@ -1,3 +1,3 @@\n fn main() {\n-    old();\n+    fresh();\n }";

struct DemoRow {
    name: &'static str,
    lang: &'static str,
    stars: u32,
}

const fn demo_row(name: &'static str, lang: &'static str, stars: u32) -> DemoRow {
    DemoRow { name, lang, stars }
}

fn demo_columns() -> Vec<TableColumn<DemoRow>> {
    vec![
        TableColumn::new("name", "Name")
            .sort_by(|row: &DemoRow| row.name)
            .cell(|row| text(row.name).nowrap()),
        TableColumn::new("lang", "Language").cell(|row: &DemoRow| text(row.lang).nowrap()),
        TableColumn::new("stars", "Stars")
            .numeric()
            .sort_by(|row: &DemoRow| row.stars)
            .cell(|row| text(row.stars.to_string()).nowrap()),
    ]
}

fn table_row(state: State) -> NodeBuilder {
    let rows = TableState::new(
        vec![
            demo_row("sigil", "Rust", 4210),
            demo_row("wgpu", "Rust", 12800),
            demo_row("helix", "Rust", 36000),
        ],
        demo_columns(),
    );
    if state == State::Selected {
        rows.select_only(1);
    }
    node(
        table(rows)
            .striped()
            .disabled(off(state))
            .viewport_height(160.0),
    )
}

fn tree_rows(state: State) -> NodeBuilder {
    let selected = state == State::Selected;
    let entry = |depth, disclosure, title: &'static str, on: bool| {
        list_row_with(
            on,
            title,
            ListRowSlots {
                depth,
                disclosure,
                ..ListRowSlots::default()
            },
        )
    };
    container()
        .layout(LayoutStyle::flex_column().fill_width().gap(2.0))
        .child(entry(0, Disclosure::Expanded, "src", false))
        .child(entry(1, Disclosure::Collapsed, "widgets", selected))
        .child(entry(1, Disclosure::Leaf, "lib.rs", false))
        .child(entry(0, Disclosure::Collapsed, "examples", false))
}

fn virtual_list_row(state: State) -> NodeBuilder {
    let items: Vec<String> = (1..=20).map(|i| format!("Item {i}")).collect();
    let selected = state == State::Selected;
    node(
        virtual_list(items, move |index, item| {
            list_row(selected && index == 1, item.clone(), format!("{index}"))
        })
        .selection(SelectionMode::Single)
        .viewport_height(140.0),
    )
}

fn list_row_row(state: State) -> NodeBuilder {
    node(list_row(state == State::Selected, "Build finished", "now"))
}

fn kv_grid_row(_state: State) -> NodeBuilder {
    node(kv_grid([("Host", "wgpu"), ("Backend", "metal")]))
}

fn kv_row_row(_state: State) -> NodeBuilder {
    node(kv_row("Region", text("eu-west-1").nowrap()))
}

fn virtual_list_fill_row(_state: State) -> NodeBuilder {
    let items: Vec<String> = (1..=20).map(|i| format!("Item {i}")).collect();
    container()
        .layout(LayoutStyle::flex_column().fill_width().height(152.0))
        .child(text("Header above the list").nowrap())
        .child(
            container()
                .layout(LayoutStyle::flex_column().fill_width().grow(1.0))
                .child(
                    virtual_list(items, |index, item| {
                        list_row(false, item.clone(), format!("{index}"))
                    })
                    .viewport_fill(),
                ),
        )
        .child(text("Footer below the list").nowrap())
}

fn chart_row(_state: State) -> NodeBuilder {
    node(
        chart([12.0, 28.0, 18.0, 34.0])
            .labels(["Q1", "Q2", "Q3", "Q4"])
            .height(160.0),
    )
}

fn heatmap_row(_state: State) -> NodeBuilder {
    node(
        container()
            .layout(LayoutStyle::flex_column().width(240.0).height(120.0))
            .child(heatmap(12, 6, |x, y| Rgba8 {
                r: 40,
                g: u8::try_from(40 + x * 16).unwrap_or(u8::MAX),
                b: u8::try_from(80 + y * 28).unwrap_or(u8::MAX),
                a: 255,
            })),
    )
}

fn diff_row(_state: State) -> NodeBuilder {
    node(
        diff(PATCH)
            .path("src/app.rs")
            .view(DiffView::Unified)
            .max_rows(24),
    )
}
