use std::hint::black_box;
use std::io;
use std::time::Instant;

use sigil::Host;
use sigil::app::FrameHooks;
use sigil::events::{EventRouter, ShellEvent};
use sigil::reactive::Runtime;
use sigil::terminal::Screen;

use crate::budgets::Scenario;
use crate::rigs::{VIEW, mount, rig};

const WARMUP: u32 = 64;
const FRAMES: u32 = 256;
const COLS: u16 = 120;
const ROWS: u16 = 40;

pub(crate) struct Sample {
    pub max_allocs: u64,
    pub mean_bytes: u64,
    pub nanos: Vec<u64>,
}

pub(crate) fn window_frame(
    host: &mut Host,
    router: &mut EventRouter,
    event: Option<ShellEvent>,
    force_emit: bool,
) {
    let hooks = FrameHooks {
        input: event.map(|event| -> sigil::app::FrameInputHook<'_> {
            Box::new(move |target: &mut Host, events: &mut EventRouter| {
                target.route(events, event);
                None
            })
        }),
        after_layout: None,
        paint: Some(Box::new(move |target, _, info| {
            if info.dirty || force_emit {
                black_box(target.emit(VIEW).commands.len());
            }
        })),
    };
    black_box(host.frame(router, VIEW, 1.0 / 60.0, hooks).intervals);
}

fn sample(mut step: impl FnMut(u32)) -> Sample {
    for frame in 0..WARMUP {
        step(frame);
    }
    let mut max_allocs = 0;
    let mut bytes = 0;
    let mut nanos = Vec::with_capacity(FRAMES as usize);
    for frame in 0..FRAMES {
        let start = Instant::now();
        let info = allocation_counter::measure(|| step(frame));
        nanos.push(u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX));
        max_allocs = max_allocs.max(info.count_total);
        bytes += info.bytes_total;
    }
    Sample {
        max_allocs,
        mean_bytes: bytes / u64::from(FRAMES),
        nanos,
    }
}

fn terminal() -> Sample {
    let mut host = Host::cells();
    let mut router = EventRouter::new();
    let mut screen = Screen::new(COLS, ROWS);
    mount(&mut host, &mut router, &rig(Scenario::Terminal));
    let mut sink = io::sink();
    sample(|_| {
        let _ = black_box(screen.frame(&mut host, &mut router, [], (COLS, ROWS)));
        let _ = screen.flush(&mut sink);
    })
}

fn window(scenario: Scenario) -> Sample {
    let rig = rig(scenario);
    let mut host = Host::new();
    let mut router = EventRouter::new();
    mount(&mut host, &mut router, &rig);
    let force = scenario == Scenario::Chart;
    sample(|frame| {
        window_frame(
            &mut host,
            &mut router,
            rig.events.map(|at| at(frame)),
            force,
        );
    })
}

pub(crate) fn run(scenario: Scenario) -> Sample {
    Runtime::scope("perf-gate", || {
        if scenario == Scenario::Terminal {
            terminal()
        } else {
            window(scenario)
        }
    })
}
