#![allow(
    unsafe_code,
    unused_crate_dependencies,
    clippy::print_stdout,
    clippy::expect_used,
    clippy::cast_precision_loss,
    reason = "the benchmark prints raw samples and uses a subset of package dependencies"
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use sigil::Host;
use sigil::events::Event;
use sigil::geometry::Point;
use sigil::kernel::HostMode;
use sigil::prelude::*;
use sigil::reactive::Owner;

struct Counting;

static ALLOCS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);

// SAFETY: every call forwards to `System` with the caller's arguments unchanged.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        // SAFETY: the caller upholds the `GlobalAlloc::alloc` contract.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller upholds the `GlobalAlloc::dealloc` contract.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

const VIEW: (f32, f32) = (1280.0, 800.0);
const NARROW: (f32, f32) = (1272.0, 800.0);
const ROWS: usize = 300;
const MESSAGES: usize = 200;
const STREAM_LIMIT: usize = 4_000;
const WARM: usize = 50;
const UNITS: usize = 400;
const HEAVY_WARM: usize = 5;
const HEAVY_UNITS: usize = 40;
const COLS: u16 = 120;
const LINES: u16 = 40;
const BODY: &str = "Layout resolves each box from its style and its constraints, and text wraps to the width that it gets from its parent.";

type Run = fn(&str);

const CELLS: &[(&str, Run)] = &[
    ("first", first),
    ("idle", idle),
    ("delta", delta),
    ("markdown", markdown_delta),
    ("all", all),
    ("hover", hover),
    ("scroll", wheel),
    ("toggle", toggle),
    ("resize", resize),
    ("grid", grid),
];

fn measure(name: &str, side: &str, warm: usize, units: usize, mut step: impl FnMut(usize)) {
    for unit in 0..warm {
        step(unit);
    }
    let mut samples = Vec::with_capacity(units);
    let allocs = ALLOCS.load(Ordering::Relaxed);
    let bytes = BYTES.load(Ordering::Relaxed);
    for unit in 0..units {
        let at = Instant::now();
        step(warm + unit);
        samples.push(at.elapsed().as_secs_f64() * 1e6);
    }
    let allocs = ALLOCS.load(Ordering::Relaxed) - allocs;
    let bytes = BYTES.load(Ordering::Relaxed) - bytes;
    samples.sort_by(f64::total_cmp);
    let count = units as f64;
    let median = samples.get(units / 2).copied().unwrap_or_default();
    println!("RAW {name} {side} us_unit {median:.2}");
    println!("RAW {name} {side} allocs_unit {:.1}", allocs as f64 / count);
    println!("RAW {name} {side} bytes_unit {:.0}", bytes as f64 / count);
}

fn chat(stream: Signal<String>) -> impl IntoNode {
    let history = (0..MESSAGES).fold(column().gap(Space::Sm), |page, index| {
        page.child(
            column()
                .child(text(format!("Message {index}")).nowrap())
                .child(text(BODY)),
        )
    });
    scroll(history.child(text(move || stream.get()))).fill()
}

fn rows(cells: Vec<Signal<u64>>) -> NodeBuilder {
    cells
        .into_iter()
        .enumerate()
        .fold(column(), |page, (index, cell)| {
            page.child(
                row()
                    .child(text(format!("Row {index}")).nowrap())
                    .child(text(move || format!("Value {}", cell.get())).nowrap())
                    .child(button(format!("Open {index}"))),
            )
        })
}

fn extend(stream: Signal<String>) {
    stream.update(|body| {
        if body.len() > STREAM_LIMIT {
            body.clear();
        }
        body.push_str("word ");
    });
}

fn frame(host: &mut Host, view: (f32, f32)) {
    black_box(host.scene(view).expect("layout").commands.len());
}

fn first(side: &str) {
    measure("first", side, HEAVY_WARM, HEAVY_UNITS, |_| {
        Owner::scope("first", || {
            let stream = signal(String::new());
            let mut host = Host::new(HostMode::Pixel);
            host.mount(chat(stream));
            frame(&mut host, VIEW);
        });
    });
}

fn idle(side: &str) {
    let stream = signal(String::new());
    let mut host = Host::new(HostMode::Pixel);
    host.mount(chat(stream));
    measure("idle", side, WARM, UNITS, |_| frame(&mut host, VIEW));
}

fn delta(side: &str) {
    let stream = signal(String::new());
    let mut host = Host::new(HostMode::Pixel);
    host.mount(chat(stream));
    measure("delta", side, WARM, UNITS, |_| {
        extend(stream);
        frame(&mut host, VIEW);
    });
}

fn markdown_delta(side: &str) {
    let stream = signal(String::new());
    let mut host = Host::new(HostMode::Pixel);
    host.mount(scroll(markdown(stream)).fill());
    measure("markdown", side, WARM, UNITS, |_| {
        extend(stream);
        frame(&mut host, VIEW);
    });
}

fn all(side: &str) {
    let cells: Vec<Signal<u64>> = (0..ROWS).map(|_| signal(0)).collect();
    let writes = cells.clone();
    let mut host = Host::new(HostMode::Pixel);
    host.mount(scroll(rows(cells)).fill());
    measure("all", side, WARM, UNITS, |unit| {
        for cell in &writes {
            cell.set(unit as u64);
        }
        frame(&mut host, VIEW);
    });
}

fn hover(side: &str) {
    let cells: Vec<Signal<u64>> = (0..ROWS).map(|_| signal(0)).collect();
    let mut host = Host::new(HostMode::Pixel);
    host.mount(scroll(rows(cells)).fill());
    frame(&mut host, VIEW);
    measure("hover", side, WARM, UNITS, |unit| {
        let y = ((unit % 30) as f32).mul_add(24.0, 12.0);
        host.route(Event::pointer_move(Point::new(600.0, y)));
        frame(&mut host, VIEW);
    });
}

fn wheel(side: &str) {
    let stream = signal(String::new());
    let mut host = Host::new(HostMode::Pixel);
    host.mount(chat(stream));
    frame(&mut host, VIEW);
    let at = Point::new(640.0, 400.0);
    host.route(Event::pointer_move(at));
    measure("scroll", side, WARM, UNITS, |unit| {
        let lines = if unit % 40 < 20 { -1.0 } else { 1.0 };
        host.route(Event::wheel(at, lines));
        frame(&mut host, VIEW);
    });
}

fn toggle(side: &str) {
    let flag = signal(false);
    let mut host = Host::new(HostMode::Pixel);
    host.mount(column().child(show(
        flag,
        || {
            let stream = signal(String::new());
            chat(stream)
        },
        || {
            let stream = signal(String::from("other"));
            chat(stream)
        },
    )));
    measure("toggle", side, HEAVY_WARM, HEAVY_UNITS, |unit| {
        flag.set(unit % 2 == 0);
        frame(&mut host, VIEW);
    });
}

fn resize(side: &str) {
    let stream = signal(String::new());
    let mut host = Host::new(HostMode::Pixel);
    host.mount(chat(stream));
    measure("resize", side, WARM, UNITS, |unit| {
        let view = if unit % 2 == 0 { NARROW } else { VIEW };
        frame(&mut host, view);
    });
}

fn grid(side: &str) {
    let cells: Vec<Signal<u64>> = (0..ROWS).map(|_| signal(0)).collect();
    let writes = cells.clone();
    let mut host = Host::new(HostMode::Cell);
    host.mount(scroll(rows(cells)).fill());
    measure("grid", side, WARM, UNITS, |unit| {
        for cell in &writes {
            cell.set(unit as u64);
        }
        black_box(host.grid([], COLS, LINES).expect("layout"));
    });
}

fn main() {
    let mut args = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"));
    let side = args.next().unwrap_or_else(|| "base".to_owned());
    let only = args.next();
    for (name, run) in CELLS {
        if only.as_deref().is_none_or(|wanted| wanted == *name) {
            Owner::scope(name, || run(&side));
        }
    }
}
