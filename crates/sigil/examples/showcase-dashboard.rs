#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;

struct Order {
    id: &'static str,
    customer: &'static str,
    total: u32,
}

fn main() -> sigil::AppResult<()> {
    sigil::run(("Sigil - dashboard", 960, 600), app)
}

fn kpi(title: &'static str, value: &'static str, trend: &'static str, tone: Tone) -> NodeBuilder {
    card(
        container()
            .layout(LayoutStyle::flex_column().gap(rows(0.5)))
            .child(label(title))
            .child(text(value))
            .child(chip(trend, tone)),
    )
}

fn column(child: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .grow(1.0)
                .flex_basis(0.0)
                .min_width(0.0),
        )
        .child(child)
}

fn kpi_row() -> NodeBuilder {
    let load = signal(0.64_f32);
    container()
        .layout(LayoutStyle::flex_row().fill_width().gap(cols(2.0)))
        .child(column(kpi("Revenue", "$48,210", "+12%", Tone::Success)))
        .child(column(kpi("Orders", "1,284", "+4%", Tone::Success)))
        .child(column(kpi("Refunds", "23", "+2", Tone::Warn)))
        .child(column(card(gauge(load).max(1.0).label("Load").warn(0.8))))
}

fn orders_table() -> TableBuilder<Order> {
    let orders = vec![
        Order {
            id: "#1042",
            customer: "Ada Lovelace",
            total: 320,
        },
        Order {
            id: "#1043",
            customer: "Alan Turing",
            total: 145,
        },
        Order {
            id: "#1044",
            customer: "Grace Hopper",
            total: 980,
        },
        Order {
            id: "#1045",
            customer: "Edsger Dijkstra",
            total: 60,
        },
    ];
    let columns = vec![
        TableColumn::new("id", "Order").cell(|row: &Order| label(row.id)),
        TableColumn::new("customer", "Customer")
            .sort_by(|row: &Order| row.customer)
            .cell(|row| label(row.customer)),
        TableColumn::new("total", "Total")
            .numeric()
            .sort_by(|row: &Order| row.total)
            .cell(|row| label(format!("${}", row.total))),
    ];
    table(TableState::new(orders, columns))
        .striped()
        .viewport_height(200.0)
}

fn sales_chart() -> PanelBuilder {
    panel().title("Weekly sales").child(
        chart([32.0, 41.0, 28.0, 52.0, 47.0, 61.0, 58.0])
            .labels(["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"])
            .height(200.0),
    )
}

fn top_bar() -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().gap(cols(2.0)).padding(cols(1.0)))
        .child(section_label("Store dashboard"))
        .child(badge("Live", Tone::Success))
        .child(avatar("TG"))
}

fn work_area() -> NodeBuilder {
    container()
        .role(Role::Main)
        .layout(
            LayoutStyle::flex_column()
                .gap(rows(1.0))
                .padding(cols(2.0))
                .fill(),
        )
        .child(kpi_row())
        .child(
            container()
                .layout(LayoutStyle::flex_row().fill_width().gap(cols(2.0)))
                .child(column(panel().title("Recent orders").child(orders_table())))
                .child(column(sales_chart())),
        )
}

pub(crate) fn app(cx: &mut Cx) -> NodeId {
    cx.add(app_shell(
        top_bar(),
        work_area(),
        status_bar([label("Synced"), label("4 orders"), label("EUR")]),
        None::<NodeBuilder>,
    ))
}
