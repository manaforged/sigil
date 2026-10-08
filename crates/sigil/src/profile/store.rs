use std::cell::RefCell;
use std::collections::VecDeque;
use std::env;
use std::path::PathBuf;
use std::process::id;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use web_time::Instant;

use serde_json::{Value, json};

use crate::profile::attr::Attribution;
use crate::profile::event::Event;
use crate::profile::key::{
    BackendStatus, Cause, Counter, CounterValue, Phase, PhaseTime, accumulate,
};

const MAX_EVENTS: usize = 65_536;
const MAX_FRAMES: usize = 4_096;
const ENV_UNSET: u8 = 0;
const ENV_OFF: u8 = 1;
const ENV_ON: u8 = 2;
const WARN_CAP: u8 = 8;

pub const FRAME_BUDGET_NS: u64 = 16_670_000;
pub const INTERACT_BUDGET_NS: u64 = 200_000_000;

static FORCED: AtomicBool = AtomicBool::new(false);
static TEST_OFF: AtomicBool = AtomicBool::new(false);
static ENV: AtomicU8 = AtomicU8::new(ENV_UNSET);
static WARNS: AtomicU8 = AtomicU8::new(0);

thread_local! {
    static STORE: RefCell<Store> = RefCell::new(Store::new());
}

#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub index: u32,
    pub cause: Cause,
    pub wall_ns: u64,
    pub phases: Vec<PhaseTime>,
    pub counts: Vec<CounterValue>,
    pub backend: Option<BackendStatus>,
    pub hot: Vec<Attribution>,
    pub gpu_passes: Vec<(&'static str, u64)>,
    pub end_us: u64,
}

impl Frame {
    #[must_use]
    pub fn counter(&self, counter: Counter) -> u64 {
        self.counts
            .iter()
            .find(|(n, _)| *n == counter)
            .map_or(0, |(_, v)| *v)
    }

    #[must_use]
    pub fn total_ns(&self) -> u64 {
        if self.wall_ns > 0 {
            return self.wall_ns;
        }
        self.phases
            .iter()
            .filter(|(phase, _)| phase.is_top_level())
            .map(|(_, ns)| *ns)
            .sum()
    }

    #[must_use]
    pub fn hottest(&self) -> Option<PhaseTime> {
        self.phases.iter().copied().max_by_key(|(_, ns)| *ns)
    }
}

struct Store {
    origin: Instant,
    events: VecDeque<Event>,
    frames: VecDeque<Frame>,
    dropped_events: u64,
    open: Frame,
    next_index: u32,
    open_start: Option<Instant>,
    next_flow: u64,
    open_flow: Option<(&'static str, u64, u64)>,
}

impl Store {
    fn new() -> Self {
        Self {
            origin: Instant::now(),
            events: VecDeque::with_capacity(1024),
            frames: VecDeque::with_capacity(64),
            dropped_events: 0,
            open: Frame::default(),
            next_index: 0,
            open_start: None,
            next_flow: 1,
            open_flow: None,
        }
    }

    fn now_us(&self) -> u64 {
        u64::try_from(self.origin.elapsed().as_micros()).unwrap_or(u64::MAX)
    }
}

pub(crate) fn enabled() -> bool {
    if TEST_OFF.load(Ordering::Relaxed) {
        return false;
    }
    if FORCED.load(Ordering::Relaxed) {
        return true;
    }
    match ENV.load(Ordering::Relaxed) {
        ENV_ON => true,
        ENV_OFF => false,
        _ => {
            let on = env_flag();
            ENV.store(if on { ENV_ON } else { ENV_OFF }, Ordering::Relaxed);
            on
        }
    }
}

fn env_flag() -> bool {
    env::var("SIGIL_PROFILE").is_ok_and(|value| !value.is_empty() && value != "0")
}

pub(crate) fn enable() {
    TEST_OFF.store(false, Ordering::Relaxed);
    FORCED.store(true, Ordering::Relaxed);
}

pub(crate) fn disable() {
    FORCED.store(false, Ordering::Relaxed);
    TEST_OFF.store(true, Ordering::Relaxed);
    ENV.store(ENV_UNSET, Ordering::Relaxed);
    reset();
}

pub(crate) fn reset() {
    WARNS.store(0, Ordering::Relaxed);
    STORE.with(|cell| *cell.borrow_mut() = Store::new());
}

pub(crate) fn dump_path() -> Option<PathBuf> {
    let value = env::var("SIGIL_PROFILE").ok()?;
    if value.is_empty() || value == "0" {
        return None;
    }
    if value == "1" {
        return Some(PathBuf::from("sigil-profile.json"));
    }
    Some(PathBuf::from(value))
}

pub(crate) fn now_us() -> u64 {
    STORE.with(|cell| cell.borrow().now_us())
}

pub(crate) fn begin_frame(backend: BackendStatus) {
    if !enabled() {
        return;
    }
    STORE.with(|cell| {
        let mut store = cell.borrow_mut();
        store.open_start = Some(Instant::now());
        store.open.cause = Cause::Idle;
        store.open.backend = Some(backend);
    });
}

pub(crate) fn complete(phase: Phase, start_us: u64, dur_ns: u64) {
    if !enabled() {
        return;
    }
    STORE.with(|cell| {
        let mut store = cell.borrow_mut();
        let micros = (dur_ns / 1_000).max(1);
        push_event(
            &mut store,
            Event::Complete {
                phase,
                tid: 1,
                start_us,
                dur_us: micros,
            },
        );
        accumulate(&mut store.open.phases, phase, dur_ns);
    });
}

pub(crate) fn count_add(counter: Counter, n: u64) {
    if !enabled() {
        return;
    }
    STORE.with(|cell| {
        accumulate(&mut cell.borrow_mut().open.counts, counter, n);
    });
}

pub(crate) fn gauge(counter: Counter, value: u64) {
    if !enabled() {
        return;
    }
    STORE.with(|cell| {
        let mut store = cell.borrow_mut();
        if let Some((_, previous)) = store
            .open
            .counts
            .iter_mut()
            .find(|(key, _)| *key == counter)
        {
            *previous = value;
        } else {
            store.open.counts.push((counter, value));
        }
    });
}

pub(crate) fn with_open(f: impl FnOnce(&mut Frame)) {
    if !enabled() {
        return;
    }
    STORE.with(|cell| f(&mut cell.borrow_mut().open));
}

pub(crate) fn interact(cause: Cause) {
    if !enabled() {
        return;
    }
    STORE.with(|cell| {
        let mut store = cell.borrow_mut();
        let ts = store.now_us();
        if cause.rank() >= store.open.cause.rank() {
            store.open.cause = cause;
        }
        let name = cause.name();
        push_event(
            &mut store,
            Event::Instant {
                name,
                cat: "input",
                ts_us: ts,
            },
        );
        if store.open_flow.is_none() {
            let id = store.next_flow;
            store.next_flow = store.next_flow.saturating_add(1);
            store.open_flow = Some((name, id, ts));
            push_event(
                &mut store,
                Event::FlowStart {
                    name,
                    id,
                    ts_us: ts,
                },
            );
        }
    });
}

pub(crate) fn tick_frame() {
    if !enabled() {
        return;
    }
    STORE.with(|cell| close_open_inner(&mut cell.borrow_mut(), true));
}

pub(crate) fn close_open() {
    if !enabled() {
        return;
    }
    STORE.with(|cell| {
        let mut store = cell.borrow_mut();
        if store.open.phases.is_empty() && store.open.counts.is_empty() {
            return;
        }
        close_open_inner(&mut store, true);
    });
}

fn close_open_inner(store: &mut Store, warn: bool) {
    let end_us = store.now_us();
    if let Some((name, id, _)) = store.open_flow.take() {
        push_event(
            store,
            Event::FlowFinish {
                name,
                id,
                ts_us: end_us,
            },
        );
    }
    let mut frame = std::mem::take(&mut store.open);
    for &(counter, value) in &frame.counts {
        push_event(
            store,
            Event::Counter {
                counter,
                ts_us: end_us,
                value,
            },
        );
    }
    frame.index = store.next_index;
    frame.end_us = end_us;
    frame.wall_ns = store.open_start.take().map_or(0, |start| {
        u64::try_from(start.elapsed().as_nanos()).unwrap_or(0)
    });
    store.next_index = store.next_index.saturating_add(1);
    if warn {
        warn_slow(&frame);
    }
    if store.frames.len() >= MAX_FRAMES {
        store.frames.pop_front();
    }
    store.frames.push_back(frame);
}

fn warn_slow(frame: &Frame) {
    if frame.total_ns() <= FRAME_BUDGET_NS || frame.cause == Cause::Idle {
        return;
    }
    if WARNS.load(Ordering::Relaxed) >= WARN_CAP {
        return;
    }
    WARNS.fetch_add(1, Ordering::Relaxed);
    #[allow(clippy::cast_precision_loss, reason = "ns → ms display only")]
    let ms = frame.total_ns() as f64 / 1_000_000.0;
    let (hot, peak) = frame
        .hottest()
        .map_or(("", 0), |(phase, ns)| (phase.name(), ns));
    #[allow(clippy::cast_precision_loss, reason = "ns → ms display only")]
    let peak_ms = peak as f64 / 1_000_000.0;
    eprintln!(
        "sigil-profile: frame {} {} {ms:.1}ms {hot}={peak_ms:.1} cmds={}",
        frame.index,
        frame.cause,
        frame.counter(Counter::SceneCmds)
    );
}

fn push_event(store: &mut Store, event: Event) {
    if store.events.len() >= MAX_EVENTS {
        store.events.pop_front();
        store.dropped_events += 1;
    }
    store.events.push_back(event);
}

pub(crate) fn frames() -> Vec<Frame> {
    STORE.with(|cell| cell.borrow().frames.iter().cloned().collect())
}

pub(crate) fn events() -> Vec<Event> {
    STORE.with(|cell| cell.borrow().events.iter().copied().collect())
}

pub(crate) fn metadata() -> Value {
    STORE.with(|cell| {
        let store = cell.borrow();
        json!({
            "schema": 1,
            "pid": id(),
            "os": env::consts::OS,
            "arch": env::consts::ARCH,
            "debug_assertions": cfg!(debug_assertions),
            "duration_us": store.now_us(),
            "total_frames": store.next_index,
            "dropped_frames": u64::from(store.next_index).saturating_sub(store.frames.len() as u64),
            "dropped_events": store.dropped_events,
            "frame_capacity": MAX_FRAMES,
            "event_capacity": MAX_EVENTS,
            "timing": "CPU wall time; GPU execution from timestamp queries when available"
        })
    })
}
