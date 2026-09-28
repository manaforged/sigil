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
                .width(ColWidth::Fixed(120.0))
                .cell(move |row: &TableRow| text_cell(row.name)),
            TableColumn::new("role", "Role")
                .sort_by(|row: &TableRow| row.role)
                .width(ColWidth::Fixed(96.0))
                .cell(move |row: &TableRow| text_cell(row.role)),
            TableColumn::new("city", "City")
                .width(ColWidth::Fixed(104.0))
                .cell(move |row: &TableRow| text_cell(row.city)),
        ],
    )
}

fn table_golden_view() -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(320.0, 172.0))
        .class("bg-surface")
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
    let img = render_widget_to_png_stub(
        |cx| cx.add(table_golden_view()),
        InteractionMode::Default,
        [320, 172],
        (1.0, 1.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("table_default"), DELTA_PCT);
}

#[test]
fn table_selected() {
    let img = render_widget_to_png_stub(
        |cx| {
            let state = table_golden_state();
            state.select_only(1);
            cx.add(
                container()
                    .layout(LayoutStyle::block().fixed_size(320.0, 172.0))
                    .class("bg-surface")
                    .child(
                        table(state)
                            .row_height(24.0)
                            .header_height(28.0)
                            .viewport_width(320.0)
                            .viewport_height(144.0),
                    ),
            )
        },
        InteractionMode::Default,
        [320, 172],
        (1.0, 1.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("table_selected"), DELTA_PCT);
}

#[test]
fn table_sorted() {
    let img = render_widget_to_png_stub(
        |cx| {
            let state = table_golden_state();
            state.toggle_sort("name");
            cx.add(
                container()
                    .layout(LayoutStyle::block().fixed_size(320.0, 172.0))
                    .class("bg-surface")
                    .child(
                        table(state)
                            .row_height(24.0)
                            .header_height(28.0)
                            .viewport_width(320.0)
                            .viewport_height(144.0),
                    ),
            )
        },
        InteractionMode::Default,
        [320, 172],
        (1.0, 1.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("table_sorted"), DELTA_PCT);
}
