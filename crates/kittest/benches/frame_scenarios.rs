#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::print_stdout, reason = "the bench prints its profile report")]

use sigil::geometry::Point;
use sigil::prelude::*;
use sigil::profile;
use sigil_kittest::{Harness, Query, input};

const SURFACE_WIDTH: u16 = 1600;
const SURFACE_HEIGHT: u16 = 1000;
const SURFACE_SCALE: f32 = 2.0;
const PAGE_ROWS: u32 = 300;
const TABLE_ROWS: u32 = 70;
const TABLE_ROW_HEIGHT: f32 = 64.0;
const TABLE_HEADER_HEIGHT: f32 = 32.0;
const MODAL_WIDTH: f32 = 1400.0;
const MODAL_BODY_HEIGHT: f32 = 800.0;
const SCROLL_TICKS: u32 = 120;
const HOVER_FRAMES: u32 = 120;
const WHEEL_LINES: f32 = 1.0;
const CAP_RANGE: std::ops::RangeInclusive<i64> = 0..=9_999;
const SCENARIO_ENV: &str = "SIGIL_SCENARIO";

type Scenario = (&'static str, fn());

const SCENARIOS: &[Scenario] = &[
    ("page_scroll", page_scroll),
    ("modal_table_scroll", modal_table_scroll),
    ("hover_sweep", hover_sweep),
];

#[derive(Clone)]
struct Product {
    title: String,
    sku: String,
    price: String,
    cap: i64,
    live: bool,
}

fn busy_page() -> impl IntoNode {
    let rows = (0..PAGE_ROWS).fold(column(), |page, index| {
        page.child(
            row()
                .child(text(format!("Row {index}")))
                .child(text(format!("Detail line for entry {index}")))
                .child(button(format!("Open {index}"))),
        )
    });
    scroll(rows).fill()
}

fn products() -> Vec<Product> {
    (0..TABLE_ROWS)
        .map(|index| Product {
            title: format!("Long product title number {index} with a descriptive suffix"),
            sku: format!("SKU-{index:05}"),
            price: format!("${}.99", 10 + index),
            cap: i64::from(index),
            live: index % 2 == 0,
        })
        .collect()
}

fn product_columns() -> Vec<TableColumn<Product>> {
    vec![
        TableColumn::new("product", "Product").cell(|row: &Product| {
            column()
                .child(text(row.title.clone()))
                .child(text(row.sku.clone()))
        }),
        TableColumn::new("price", "Price").cell(|row: &Product| text(row.price.clone())),
        TableColumn::new("cap", "Cap")
            .cell(|row: &Product| number_field(signal(row.cap), CAP_RANGE).text_field()),
        TableColumn::new("live", "Live").cell(|row: &Product| toggle(signal(row.live))),
    ]
}

fn modal_page() -> impl IntoNode {
    let table_view = table(TableState::new(products(), product_columns()))
        .row_height(TABLE_ROW_HEIGHT)
        .header_height(TABLE_HEADER_HEIGHT)
        .viewport_fill()
        .selectable_checkboxes();
    container()
        .layout(LayoutStyle::flex_column().fill_width().fill_height())
        .child(busy_page())
        .child(
            modal(signal(true))
                .title("Catalog")
                .width(MODAL_WIDTH)
                .child(
                    container()
                        .layout(
                            LayoutStyle::flex_column()
                                .fill_width()
                                .height(MODAL_BODY_HEIGHT),
                        )
                        .child(table_view),
                ),
        )
}

fn surface(app: impl IntoNode + 'static) -> Harness {
    let mut ui = Harness::new(app).gpu(
        u32::from(SURFACE_WIDTH),
        u32::from(SURFACE_HEIGHT),
        SURFACE_SCALE,
    );
    ui.settle();
    ui
}

fn center() -> Point {
    Point::new(
        f32::from(SURFACE_WIDTH) / 2.0,
        f32::from(SURFACE_HEIGHT) / 2.0,
    )
}

fn wheel(ui: &mut Harness, at: Point) {
    let half = SCROLL_TICKS / 2;
    for tick in 0..SCROLL_TICKS {
        let lines = if tick < half {
            -WHEEL_LINES
        } else {
            WHEEL_LINES
        };
        ui.send([
            input::pointer_move(at.x, at.y),
            input::scroll_lines(at.x, at.y, lines),
        ]);
    }
}

fn measured(name: &str, ui: &mut Harness, run: impl FnOnce(&mut Harness)) {
    profile::enable();
    profile::reset();
    run(ui);
    let frames = profile::frames();
    let idle = frames.iter().all(|frame| {
        frame.counter(profile::Counter::GpuPlanUnchanged) > 0
            || frame.counter(profile::Counter::SceneCmds) == 0
                && frame.counter(profile::Counter::GpuRenderPasses) == 0
    });
    println!("== scenario {name} ==");
    if idle {
        println!(
            "WARNING: scenario {name} changed nothing on screen; its input did not reach the UI"
        );
    }
    println!("{}", profile::summary());
    profile::disable();
}

fn page_scroll() {
    let mut ui = surface(busy_page());
    measured("page_scroll", &mut ui, |ui| wheel(ui, center()));
}

fn modal_table_scroll() {
    let mut ui = surface(modal_page());
    measured("modal_table_scroll", &mut ui, |ui| wheel(ui, center()));
}

fn hover_sweep() {
    let mut ui = surface(busy_page());
    let height = f32::from(SURFACE_HEIGHT);
    let targets: Vec<Point> = ui
        .find_all(&Query::role(Role::Button))
        .iter()
        .filter_map(sigil_kittest::Node::center)
        .filter(|at| at.y >= 0.0 && at.y < height)
        .collect();
    measured("hover_sweep", &mut ui, |ui| {
        for (_, at) in (0..HOVER_FRAMES).zip(targets.iter().cycle()) {
            ui.send([input::pointer_move(at.x, at.y)]);
        }
    });
}

fn main() {
    let only = std::env::var(SCENARIO_ENV).ok();
    let mut ran = false;
    for (name, run) in SCENARIOS {
        if only.as_deref().is_none_or(|wanted| wanted == *name) {
            sigil::reactive::Runtime::scope(name, run);
            ran = true;
        }
    }
    if !ran {
        let names: Vec<&str> = SCENARIOS.iter().map(|(name, _)| *name).collect();
        eprintln!("{SCENARIO_ENV}: no scenario named {only:?}; known: {names:?}");
        std::process::exit(2);
    }
}
