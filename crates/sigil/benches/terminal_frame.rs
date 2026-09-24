#![allow(
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
use sigil::Host;
use sigil::events::EventRouter;
use sigil::kernel::{Cx, NodeId, container};
use sigil::reactive::Runtime;
use sigil::style::{Tone, grid};
use sigil::terminal::{ColorDepth, Grid, step as try_step};
use sigil::widgets::{ColWidth, TableColumn, TableState, badge, label, table};

const COLS: u16 = 120;
const ROWS: u16 = 40;
const CELL: f32 = 8.0;
const TRUECOLOR: ColorDepth = ColorDepth::Truecolor;
const VIEW: (f32, f32) = (960.0, 320.0);

fn step(
    host: &mut Host,
    router: &mut EventRouter,
    events: impl IntoIterator<Item = sigil::hal::ShellEvent>,
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

    group.bench_function("idle/200-rows", |b| {
        Runtime::scope("bench-idle", || {
            let mut host = Host::cells();
            let mut router = EventRouter::new();
            let mut screen = sigil::terminal::Screen::new(COLS, ROWS);
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
            let mut screen = sigil::terminal::Screen::new(COLS, ROWS);
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

    group.bench_function("phase/focus", |b| {
        Runtime::scope("bench-focus", || {
            grid(CELL);
            let mut host = Host::cells();
            let mut router = EventRouter::new();
            mount(&mut host, 200);
            drop(step(&mut host, &mut router, [], COLS, ROWS));
            b.iter(|| host.repair_focus(black_box(&mut router)));
        });
    });

    group.bench_function("phase/scene", |b| {
        Runtime::scope("bench-scene", || {
            grid(CELL);
            let mut host = Host::cells();
            let mut router = EventRouter::new();
            mount(&mut host, 200);
            drop(step(&mut host, &mut router, [], COLS, ROWS));
            b.iter(|| black_box(host.emit(VIEW).commands.len()));
        });
    });

    group.bench_function("phase/raster", |b| {
        Runtime::scope("bench-raster", || {
            grid(CELL);
            let mut host = Host::cells();
            let mut router = EventRouter::new();
            mount(&mut host, 200);
            drop(step(&mut host, &mut router, [], COLS, ROWS));
            let scene = host.emit(VIEW);
            let theme = host.theme();
            b.iter(|| {
                black_box(sigil::terminal::raster(&scene, theme, COLS, ROWS, CELL));
            });
        });
    });

    group.finish();
}

fn allocations(_c: &mut Criterion) {
    Runtime::scope("bench-allocs", || {
        grid(CELL);
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        let mut screen = sigil::terminal::Screen::new(COLS, ROWS);
        mount(&mut host, 200);
        for _ in 0..10 {
            screen
                .frame(&mut host, &mut router, [], (COLS, ROWS))
                .expect("layout");
            screen.flush(&mut io::sink()).expect("flush");
        }

        let scene = host.emit(VIEW);
        let node_count = scene.commands.len();
        let text_count = scene
            .commands
            .iter()
            .filter(|command| matches!(command, sigil::scene::SceneCommand::GlyphRun(_)))
            .count();
        let paint = allocation_counter::measure(|| {
            black_box(host.emit(VIEW));
        });
        let mut cells = Grid::new(COLS, ROWS);
        let raster = allocation_counter::measure(|| {
            sigil::terminal::raster_into(&mut cells, &scene, host.theme(), CELL);
        });
        let layout = allocation_counter::measure(|| {
            host.layout((f32::from(COLS) * CELL, f32::from(ROWS) * CELL))
                .expect("layout");
        });
        let focus = allocation_counter::measure(|| host.repair_focus(&mut router));
        let frame = allocation_counter::measure(|| {
            screen
                .frame(&mut host, &mut router, [], (COLS, ROWS))
                .expect("layout");
        });
        println!(
            "\ncommands={node_count} text={text_count}\n  emit        allocs={} bytes={}\n  raster_into allocs={} bytes={}\n  layout allocs={} bytes={}\n  focus allocs={} bytes={}\n  screen.frame allocs={} bytes={}\n",
            paint.count_total,
            paint.bytes_total,
            raster.count_total,
            raster.bytes_total,
            layout.count_total,
            layout.bytes_total,
            focus.count_total,
            focus.bytes_total,
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
                let _: () =
                    sigil::terminal::render(&mut sink, &same, &frame, TRUECOLOR).expect("write");
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
                let _: () =
                    sigil::terminal::render(&mut sink, &blank, &frame, TRUECOLOR).expect("write");
                black_box(());
            });
        });
    });
    group.finish();
}

criterion_group!(benches, frame, allocations, write);
criterion_main!(benches);
