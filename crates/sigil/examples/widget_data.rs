#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::widgets::TableBuilder;

struct Repo {
    name: &'static str,
    stars: u32,
}

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - data").size(520, 720), build)
}

fn virtual_list_demo() -> NodeBuilder {
    let items: Vec<String> = (1..=50).map(|i| format!("Item {i}")).collect();
    container().child(
        virtual_list(items, |index, item| list_row(index == 0, item.clone(), ""))
            .selection(SelectionMode::Single)
            .viewport_height(200.0),
    )
}

fn table_demo() -> TableBuilder<Repo> {
    let repos = vec![
        Repo {
            name: "sigil",
            stars: 4210,
        },
        Repo {
            name: "wgpu",
            stars: 12800,
        },
    ];
    let columns = vec![
        TableColumn::new("name", "Name")
            .sort_by(|row: &Repo| row.name)
            .cell(|row| text(row.name).nowrap()),
        TableColumn::new("stars", "Stars")
            .numeric()
            .sort_by(|row: &Repo| row.stars)
            .cell(|row| text(row.stars.to_string()).nowrap()),
    ];
    table(TableState::new(repos, columns))
        .striped()
        .viewport_height(160.0)
}

fn heatmap_demo() -> NodeBuilder {
    heatmap(12, 6, |x, y| Rgba8 {
        r: 40,
        g: u8::try_from(x * 20).unwrap_or(u8::MAX),
        b: u8::try_from(y * 40).unwrap_or(u8::MAX),
        a: 255,
    })
}

fn list_row_demo() -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column())
        .child(list_row(true, "Inbox", "12"))
        .child(list_row(false, "Archive", "340"))
}

fn kv_grid_demo() -> NodeBuilder {
    kv_grid([("Region", "eu-west"), ("Replicas", "3")])
}

fn kv_row_demo() -> NodeBuilder {
    kv_row("Status", chip("Healthy", Tone::Success))
}

fn build() -> impl IntoNode {
    scroll(
        container()
            .role(Role::Main)
            .layout(LayoutStyle::flex_column().gap(rows(1.0)).padding(cols(2.0)))
            .child(table_demo())
            .child(virtual_list_demo())
            .child(heatmap_demo())
            .child(list_row_demo())
            .child(kv_grid_demo())
            .child(kv_row_demo()),
    )
    .fill()
}
