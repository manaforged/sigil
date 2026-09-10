//! Terminal frame cost: layout, paint walk, raster, and the diffed write.
//!
//! The workload is a table of 200 rows on a 120x40 grid — a process list on
//! a full-screen terminal. `steady` re-renders an unchanged tree, which is
//! what an idle app does 30 times a second.

#![allow(
    missing_docs,
    unused_crate_dependencies,
    clippy::panic,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::too_many_lines,
    reason = "the benchmark reports measurements and uses a subset of package dependencies"
)]

use std::hint::black_box;
use std::io;
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use sigil_kernel::{Cx, NodeId, container};
use sigil_reactive::Runtime;
use sigil_style::{Tone, grid};
use sigil_tui::{EventRouter, Grid, Host, step as try_step};
use sigil_widgets::{ColWidth, TableColumn, TableState, badge, label, table};

const COLS: u16 = 120;
const ROWS: u16 = 40;
const CELL: f32 = 8.0;

fn step(
    host: &mut Host,
    router: &mut EventRouter,
    events: impl IntoIterator<Item = sigil_hal::ShellEvent>,
    cols: u16,
    rows: u16,
) -> Grid {
    try_step(host, router, events, cols, rows).expect("layout")
}

#[derive(Clone)]
struct Row {
    pid: u32,
    port: u16,
    place: String,
    cmd: String,
}

/// The same shape as `rows`, with every cell's text different per `turn`.
fn churned(n: u32, turn: u32) -> Vec<Row> {
    (0..n)
        .map(|i| Row {
            pid: 1000 + i,
            port: 5000 + u16::try_from(i % 900).unwrap_or(0),
            place: format!("repo-{}-{turn}", i % 17),
            cmd: format!("node /dev/project-{i}/vite {turn}"),
        })
        .collect()
}

fn rows(n: u32) -> Vec<Row> {
    (0..n)
        .map(|i| Row {
            pid: 1000 + i,
            port: 5000 + u16::try_from(i % 900).unwrap_or(0),
            place: format!("repo-{}", i % 17),
            cmd: format!("node /home/dev/project-{i}/node_modules/.bin/vite"),
        })
        .collect()
}

fn columns() -> Vec<TableColumn<Row>> {
    vec![
        TableColumn::new("pid", "PID")
            .width(ColWidth::Fixed(56.0))
            .sort_by(|row: &Row| row.pid)
            .cell(|row: &Row| label(row.pid.to_string())),
        TableColumn::new("port", "PORT")
            .width(ColWidth::Fixed(72.0))
            .cell(|row: &Row| label(row.port.to_string())),
        TableColumn::new("state", "STATE")
            .width(ColWidth::Fixed(72.0))
            .cell(|_: &Row| badge("STRAY", Tone::Warn)),
        TableColumn::new("where", "WHERE")
            .width(ColWidth::Fixed(176.0))
            .cell(|row: &Row| label(row.place.clone())),
        TableColumn::new("cmd", "COMMAND")
            .width(ColWidth::Fraction(1.0))
            .cell(|row: &Row| label(row.cmd.clone())),
    ]
}

fn mount(host: &mut Host, n: u32) {
    let data = rows(n);
    host.mount(move |cx: &mut Cx| -> NodeId {
        let state = TableState::new(data.clone(), columns());
        cx.add(
            container().child(
                table(state)
                    .row_height(CELL)
                    .header_height(CELL)
                    .viewport_height(f32::from(ROWS) * CELL),
            ),
        )
    });
}

fn frame(c: &mut Criterion) {
    let mut group = c.benchmark_group("tui-frame");
    group.measurement_time(Duration::from_secs(5));

    // Two frames worth measuring: one where nothing changed, one where every
    // row did. Both flush, because a host that never flushes takes a branch
    // through Screen that the live loop never does.
    group.bench_function("idle/200-rows", |b| {
        Runtime::scope("bench-idle", || {
            let mut host = Host::cells();
            let mut router = EventRouter::new();
            let mut screen = sigil_tui::Screen::new(COLS, ROWS);
            mount(&mut host, 200);
            let mut sink = io::sink();
            screen
                .frame(&mut host, &mut router, [], (COLS, ROWS))
                .expect("layout");
            screen.flush(&mut sink).expect("flush");
            b.iter(|| {
                screen
                    .frame(&mut host, &mut router, [], (COLS, ROWS))
                    .expect("layout");
                screen.flush(&mut sink).expect("flush");
                black_box(());
            });
        });
    });

    group.bench_function("churn/200-rows", |b| {
        Runtime::scope("bench-churn", || {
            let mut host = Host::cells();
            let mut router = EventRouter::new();
            let mut screen = sigil_tui::Screen::new(COLS, ROWS);
            let state = TableState::new(rows(200), columns());
            let mounted = state.clone();
            host.mount(move |cx: &mut Cx| -> NodeId {
                cx.add(
                    container().child(
                        table(mounted.clone())
                            .row_height(CELL)
                            .header_height(CELL)
                            .viewport_height(f32::from(ROWS) * CELL),
                    ),
                )
            });
            let mut sink = io::sink();
            screen
                .frame(&mut host, &mut router, [], (COLS, ROWS))
                .expect("layout");
            screen.flush(&mut sink).expect("flush");
            let mut turn = 0_u32;
            b.iter(|| {
                // Same row count each time, so this measures churn rather
                // than a tree that shrank.
                turn = turn.wrapping_add(1);
                state.replace_rows(churned(200, turn));
                screen
                    .frame(&mut host, &mut router, [], (COLS, ROWS))
                    .expect("layout");
                screen.flush(&mut sink).expect("flush");
                black_box(());
            });
        });
    });

    group.bench_function("phase/layout", |b| {
        Runtime::scope("bench-layout", || {
            grid(CELL);
            let mut host = Host::cells();
            let mut router = EventRouter::new();
            mount(&mut host, 200);
            drop(step(&mut host, &mut router, [], COLS, ROWS));
            b.iter(|| {
                black_box(host.layout((f32::from(COLS) * CELL, f32::from(ROWS) * CELL)))
                    .expect("layout");
            });
        });
    });

    group.bench_function("phase/painted", |b| {
        Runtime::scope("bench-painted", || {
            grid(CELL);
            let mut host = Host::cells();
            let mut router = EventRouter::new();
            mount(&mut host, 200);
            drop(step(&mut host, &mut router, [], COLS, ROWS));
            let mut nodes = Vec::new();
            host.paint_into(&mut nodes);
            b.iter(|| {
                host.paint_into(&mut nodes);
                black_box(nodes.len());
            });
        });
    });

    group.bench_function("phase/raster", |b| {
        Runtime::scope("bench-raster", || {
            grid(CELL);
            let mut host = Host::cells();
            let mut router = EventRouter::new();
            mount(&mut host, 200);
            drop(step(&mut host, &mut router, [], COLS, ROWS));
            let nodes = host.painted();
            b.iter(|| {
                black_box(sigil_tui::raster(&nodes, COLS, ROWS, CELL));
            });
        });
    });

    group.finish();
}

/// Print allocation counts per phase. Criterion measures time; this says
/// whether the time is allocation.
fn allocations(_c: &mut Criterion) {
    Runtime::scope("bench-allocs", || {
        grid(CELL);
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        let mut screen = sigil_tui::Screen::new(COLS, ROWS);
        mount(&mut host, 200);
        screen
            .frame(&mut host, &mut router, [], (COLS, ROWS))
            .expect("layout");

        let mut nodes = Vec::new();
        host.paint_into(&mut nodes);
        let node_count = nodes.len();
        let text_count = nodes.iter().filter(|n| n.text.is_some()).count();

        let paint = allocation_counter::measure(|| {
            host.paint_into(&mut nodes);
        });
        let mut cells = Grid::new(COLS, ROWS);
        let raster = allocation_counter::measure(|| {
            sigil_tui::raster_into(&mut cells, &nodes, CELL);
        });
        let frame = allocation_counter::measure(|| {
            screen
                .frame(&mut host, &mut router, [], (COLS, ROWS))
                .expect("layout");
        });
        println!(
            "\nnodes={node_count} text={text_count}\n  paint_into  allocs={} bytes={}\n  raster_into allocs={} bytes={}\n  screen.frame allocs={} bytes={}\n",
            paint.count_total,
            paint.bytes_total,
            raster.count_total,
            raster.bytes_total,
            frame.count_total,
            frame.bytes_total,
        );
    });
}

fn write(c: &mut Criterion) {
    let mut group = c.benchmark_group("tui-write");
    group.bench_function("diff-unchanged", |b| {
        Runtime::scope("bench-write", || {
            grid(CELL);
            let mut host = Host::cells();
            let mut router = EventRouter::new();
            mount(&mut host, 200);
            let frame = step(&mut host, &mut router, [], COLS, ROWS);
            let same = frame.clone();
            b.iter(|| {
                let mut sink = io::sink();
                let _: () = sigil_tui::render(&mut sink, &same, &frame).expect("write");
                black_box(());
            });
        });
    });
    group.bench_function("full-redraw", |b| {
        Runtime::scope("bench-write-full", || {
            grid(CELL);
            let mut host = Host::cells();
            let mut router = EventRouter::new();
            mount(&mut host, 200);
            let frame = step(&mut host, &mut router, [], COLS, ROWS);
            let blank = Grid::new(COLS, ROWS);
            b.iter(|| {
                let mut sink = io::sink();
                let _: () = sigil_tui::render(&mut sink, &blank, &frame).expect("write");
                black_box(());
            });
        });
    });
    group.finish();
}

criterion_group!(benches, frame, allocations, write);
criterion_main!(benches);
