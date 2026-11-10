use super::*;

#[derive(Clone)]
struct TableRow {
    name: &'static str,
    role: &'static str,
    city: &'static str,
}

fn table_golden_state() -> sigil::widgets::TableState<TableRow> {
    let rows = vec![
        TableRow {
            name: "Ada",
            role: "Engineer",
            city: "London",
        },
        TableRow {
            name: "Grace",
            role: "Admiral",
            city: "Arlington",
        },
        TableRow {
            name: "Edsger",
            role: "Scientist",
            city: "Nuenen",
        },
        TableRow {
            name: "Barbara",
            role: "Architect",
            city: "Cambridge",
        },
    ];
    let text_cell = |value: &'static str| container().child(text(value));
    sigil::widgets::TableState::new(
        rows,
        vec![
            TableColumn::new("name", "Name")
                .sort_by(|row: &TableRow| row.name)
                .width(ColumnWidth::Fixed(120.0))
                .cell(move |row: &TableRow| text_cell(row.name)),
            TableColumn::new("role", "Role")
                .sort_by(|row: &TableRow| row.role)
                .width(ColumnWidth::Fixed(96.0))
                .cell(move |row: &TableRow| text_cell(row.role)),
            TableColumn::new("city", "City")
                .width(ColumnWidth::Fixed(104.0))
                .cell(move |row: &TableRow| text_cell(row.city)),
        ],
    )
}

fn table_golden_view() -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(320.0, 172.0))
        .background(Token::Surface)
        .child(
            table(table_golden_state())
                .row_height(24.0)
                .header_height(28.0)
                .viewport_width(320.0)
                .viewport_height(144.0),
        )
}

#[test]
fn table_default() {
    let img = Harness::render(table_golden_view)
        .stub()
        .mode(InteractionMode::Default)
        .size([320, 172])
        .hit((1.0, 1.0))
        .image();
    Golden::new("table_default").delta(DELTA_PCT).assert(img);
}

#[test]
fn table_selected() {
    let img = Harness::render(|| {
        let state = table_golden_state();
        state.select_only(1);
        container()
            .layout(LayoutStyle::block().fixed_size(320.0, 172.0))
            .background(Token::Surface)
            .child(
                table(state)
                    .row_height(24.0)
                    .header_height(28.0)
                    .viewport_width(320.0)
                    .viewport_height(144.0),
            )
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([320, 172])
    .hit((1.0, 1.0))
    .image();
    Golden::new("table_selected").delta(DELTA_PCT).assert(img);
}

#[test]
fn table_sorted() {
    let img = Harness::render(|| {
        let state = table_golden_state();
        state.toggle_sort("name");
        container()
            .layout(LayoutStyle::block().fixed_size(320.0, 172.0))
            .background(Token::Surface)
            .child(
                table(state)
                    .row_height(24.0)
                    .header_height(28.0)
                    .viewport_width(320.0)
                    .viewport_height(144.0),
            )
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([320, 172])
    .hit((1.0, 1.0))
    .image();
    Golden::new("table_sorted").delta(DELTA_PCT).assert(img);
}
