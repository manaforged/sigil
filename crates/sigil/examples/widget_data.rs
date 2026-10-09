#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::widgets::TableBuilder;

pub(crate) struct Repo {
    name: &'static str,
    lang: &'static str,
    stars: u32,
}

const fn repo(name: &'static str, lang: &'static str, stars: u32) -> Repo {
    Repo { name, lang, stars }
}

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - data").size(520, 720), build)
}

fn repo_columns() -> Vec<TableColumn<Repo>> {
    vec![
        TableColumn::new("name", "Name")
            .sort_by(|row: &Repo| row.name)
            .cell(|row| text(row.name).nowrap()),
        TableColumn::new("lang", "Language").cell(|row: &Repo| text(row.lang).nowrap()),
        TableColumn::new("stars", "Stars")
            .numeric()
            .sort_by(|row: &Repo| row.stars)
            .cell(|row| text(row.stars.to_string()).nowrap()),
    ]
}

pub(crate) fn table_demo(selected: bool) -> TableBuilder<Repo> {
    let rows = TableState::new(
        vec![
            repo("sigil", "Rust", 4210),
            repo("wgpu", "Rust", 12800),
            repo("helix", "Rust", 36000),
        ],
        repo_columns(),
    );
    if selected {
        rows.select_only(1);
    }
    table(rows).striped().viewport_height(160.0)
}

pub(crate) fn virtual_list_demo(selected: bool) -> impl IntoNode + 'static {
    let items: Vec<String> = (1..=20).map(|i| format!("Item {i}")).collect();
    virtual_list(items, move |index, item| {
        list_row(selected && index == 1, item.clone(), format!("{index}"))
    })
    .selection(SelectionMode::Single)
    .viewport_height(140.0)
}

pub(crate) fn virtual_list_fill_demo() -> NodeBuilder {
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

pub(crate) fn heatmap_demo() -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().width(240.0).height(120.0))
        .child(heatmap(12, 6, |x, y| Rgba8 {
            r: 40,
            g: u8::try_from(40 + x * 16).unwrap_or(u8::MAX),
            b: u8::try_from(80 + y * 28).unwrap_or(u8::MAX),
            a: 255,
        }))
}

pub(crate) fn list_row_demo(selected: bool) -> NodeBuilder {
    list_row(selected, "Build finished", "now")
}

pub(crate) fn list_row_tree_demo(selected: bool) -> NodeBuilder {
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

pub(crate) fn kv_grid_demo() -> NodeBuilder {
    kv_grid([("Host", "wgpu"), ("Backend", "metal")])
}

pub(crate) fn kv_row_demo() -> NodeBuilder {
    kv_row("Region", text("eu-west-1").nowrap())
}

fn build() -> impl IntoNode {
    scroll(
        container()
            .role(Role::Main)
            .layout(LayoutStyle::flex_column().gap(rows(1.0)).padding(cols(2.0)))
            .child(table_demo(false))
            .child(virtual_list_demo(false))
            .child(virtual_list_fill_demo())
            .child(heatmap_demo())
            .child(list_row_demo(false))
            .child(list_row_tree_demo(false))
            .child(kv_grid_demo())
            .child(kv_row_demo()),
    )
    .fill()
}
