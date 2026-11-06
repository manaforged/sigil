use std::cell::RefCell;
use std::collections::VecDeque;
use std::env;
use std::path::PathBuf;
use std::process::id;
use std::sync::atomic::{AtomicU8, Ordering};
use web_time::Instant;

use serde_json::{Value, json};

use crate::profile::attr::Attribution;
use crate::profile::event::TraceEvent;
use crate::profile::key::{
    BackendStatus, Cause, Counter, CounterValue, Hotspot, Phase, PhaseTime, accumulate,
};
use crate::profile::site::{Site, SiteKind};

const MAX_EVENTS: usize = 65_536;
const MAX_FRAMES: usize = 4_096;
const ENV_UNSET: u8 = 0;
const ENV_OFF: u8 = 1;
const ENV_ON: u8 = 2;
const WARN_CAP: u8 = 8;
const SITE_SLICE_NS: u64 = 50_000;

kittest_pub! {
    /// The budget of one 60 Hz frame in nanoseconds.
    #[doc(hidden)]
    pub const FRAME_BUDGET_NS: u64 = 16_670_000;
}
/// The budget of one interaction in nanoseconds.
pub(crate) const INTERACT_BUDGET_NS: u64 = 200_000_000;

static STATE: AtomicU8 = AtomicU8::new(ENV_UNSET);
static WARNS: AtomicU8 = AtomicU8::new(0);

thread_local! {
    static STORE: RefCell<Store> = RefCell::new(Store::new());
}

/// Holds the recorded timings, counters, attributions, and GPU pass times of one frame.
#[derive(Clone, Debug, Default)]
pub struct Frame {
    /// The index.
    pub index: u32,
    /// The cause.
    pub cause: Cause,
    /// The wall ns.
    pub wall_ns: u64,
    /// The phases.
    pub phases: Vec<PhaseTime>,
    /// The counts.
    pub counts: Vec<CounterValue>,
    /// The backend, if any.
    pub backend: Option<BackendStatus>,
    /// The hot.
    pub hot: Vec<Attribution>,
    /// The gpu passes.
    pub gpu_passes: Vec<(&'static str, u64)>,
    /// The time each effect and child list line took, slowest first once the frame closes.
    pub sites: Vec<Site>,
    /// The end us.
    pub end_us: u64,
    /// The spans and site runs recorded in this frame.
    pub zones: u32,
    /// The estimated time the profiler itself added to this frame.
    pub overhead_ns: u64,
}

impl Frame {
    /// Returns the value of `counter` in this frame.
    #[must_use]
    pub fn counter(&self, counter: Counter) -> u64 {
        self.counts
            .iter()
            .find(|(n, _)| *n == counter)
            .map_or(0, |(_, v)| *v)
    }

    /// Returns the total ns.
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

    /// Returns the frame time minus the time spent waiting for the GPU or display.
    #[must_use]
    pub fn work_ns(&self) -> u64 {
        let wait = self
            .phases
            .iter()
            .filter(|(phase, _)| *phase == Phase::GpuWait)
            .map(|(_, ns)| *ns)
            .sum();
        self.total_ns().saturating_sub(wait)
    }

    /// Returns the working phase that took longest, preferring nested phases over their parents.
    #[must_use]
    pub fn hottest(&self) -> Option<PhaseTime> {
        let wait = self.total_ns().saturating_sub(self.work_ns());
        let working = || {
            self.phases
                .iter()
                .copied()
                .filter(|(phase, _)| *phase != Phase::GpuWait)
                .map(move |(phase, ns)| match phase {
                    Phase::Paint | Phase::RenderCpu => (phase, ns.saturating_sub(wait)),
                    _ => (phase, ns),
                })
        };
        working()
            .filter(|(phase, _)| !phase.is_top_level())
            .max_by_key(|(_, ns)| *ns)
            .or_else(|| working().max_by_key(|(_, ns)| *ns))
    }
}

struct Store {
    origin: Instant,
    events: VecDeque<TraceEvent>,
    frames: VecDeque<Frame>,
    dropped_events: u64,
    open: Frame,
    next_index: u32,
    open_start: Option<Instant>,
    next_flow: u64,
    open_flow: Option<(&'static str, u64, u64)>,
    site_stack: Vec<u64>,
    site_index: ahash::AHashMap<(usize, SiteKind), usize>,
    attr_index: ahash::AHashMap<(Hotspot, u64), usize>,
    attr_distinct: ahash::AHashMap<Hotspot, usize>,
    open_spans: u32,
    open_sites: u32,
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
            site_stack: Vec::new(),
            site_index: ahash::AHashMap::new(),
            attr_index: ahash::AHashMap::new(),
            attr_distinct: ahash::AHashMap::new(),
            open_spans: 0,
            open_sites: 0,
        }
    }

    fn now_us(&self) -> u64 {
        u64::try_from(self.origin.elapsed().as_micros()).unwrap_or(u64::MAX)
    }

    fn since_origin_us(&self, at: Instant) -> u64 {
        u64::try_from(at.saturating_duration_since(self.origin).as_micros()).unwrap_or(u64::MAX)
    }

    fn push(&mut self, event: TraceEvent) {
        if self.events.len() >= MAX_EVENTS {
            self.events.pop_front();
            self.dropped_events += 1;
        }
        self.events.push_back(event);
    }
}

pub(crate) fn enabled() -> bool {
    match STATE.load(Ordering::Relaxed) {
        ENV_ON => true,
        ENV_OFF => false,
        _ => {
            let on = dump_path().is_some();
            STATE.store(if on { ENV_ON } else { ENV_OFF }, Ordering::Relaxed);
            on
        }
    }
}

pub(crate) fn enable() {
    STATE.store(ENV_ON, Ordering::Relaxed);
}

pub(crate) fn disable() {
    STATE.store(ENV_OFF, Ordering::Relaxed);
    reset();
}

pub(crate) fn reset() {
    WARNS.store(0, Ordering::Relaxed);
    STORE.with(|cell| *cell.borrow_mut() = Store::new());
}

pub(crate) fn dump_path() -> Option<PathBuf> {
    let value = env::var("SIGIL_PROFILE").ok()?;
    match value.as_str() {
        "" | "0" => None,
        "1" => Some(PathBuf::from("sigil-profile.json")),
        _ => Some(PathBuf::from(value)),
    }
}

fn with_store(f: impl FnOnce(&mut Store)) {
    if enabled() {
        STORE.with(|cell| f(&mut cell.borrow_mut()));
    }
}

pub(crate) fn begin_frame(backend: BackendStatus) {
    with_store(|store| {
        store.open_start = Some(Instant::now());
        store.open.cause = Cause::Idle;
        store.open.backend = Some(backend);
    });
}

pub(crate) fn complete(phase: Phase, start: Instant, dur_ns: u64) {
    with_store(|store| {
        let start_us = store.since_origin_us(start);
        store.open_spans = store.open_spans.saturating_add(1);
        store.push(TraceEvent::Complete {
            phase,
            tid: 1,
            start_us,
            dur_us: (dur_ns / 1_000).max(1),
        });
        accumulate(&mut store.open.phases, phase, dur_ns);
    });
}

pub(crate) fn site_enter() {
    with_store(|store| store.site_stack.push(0));
}

pub(crate) fn site_exit(
    location: &'static core::panic::Location<'static>,
    kind: SiteKind,
    start: Instant,
    total_ns: u64,
) {
    with_store(|store| {
        let nested = store.site_stack.pop().unwrap_or(0);
        store.open_sites = store.open_sites.saturating_add(1);
        if let Some(parent) = store.site_stack.last_mut() {
            *parent = parent.saturating_add(total_ns);
        }
        let self_ns = total_ns.saturating_sub(nested);
        let key = (core::ptr::from_ref(location) as usize, kind);
        let sites = &mut store.open.sites;
        let index = *store.site_index.entry(key).or_insert_with(|| {
            sites.push(Site {
                location,
                kind,
                self_ns: 0,
                total_ns: 0,
                runs: 0,
            });
            sites.len() - 1
        });
        let site = &mut sites[index];
        site.self_ns = site.self_ns.saturating_add(self_ns);
        site.total_ns = site.total_ns.saturating_add(total_ns);
        site.runs = site.runs.saturating_add(1);
        if total_ns >= SITE_SLICE_NS {
            let start_us = store.since_origin_us(start);
            store.push(TraceEvent::Site {
                location,
                kind,
                start_us,
                dur_us: total_ns / 1_000,
            });
        }
    });
}

pub(crate) fn attribute(hotspot: Hotspot, n: u64, key: u64, subject: impl FnOnce(usize) -> String) {
    with_store(|store| {
        let hot = &mut store.open.hot;
        let distinct = &mut store.attr_distinct;
        let index = *store.attr_index.entry((hotspot, key)).or_insert_with(|| {
            let seen = distinct.entry(hotspot).or_insert(0);
            let name = subject(*seen);
            hot.iter()
                .position(|a| a.hotspot == hotspot && a.subject == name)
                .unwrap_or_else(|| {
                    *seen += 1;
                    hot.push(Attribution {
                        hotspot,
                        subject: name,
                        count: 0,
                    });
                    hot.len() - 1
                })
        });
        let entry = &mut hot[index];
        entry.count = entry.count.saturating_add(n);
    });
}

pub(crate) fn count_add(counter: Counter, n: u64) {
    with_store(|store| accumulate(&mut store.open.counts, counter, n));
}

#[cfg(feature = "scene")]
pub(crate) fn gauge(counter: Counter, value: u64) {
    with_store(|store| {
        let counts = &mut store.open.counts;
        match counts.iter_mut().find(|(key, _)| *key == counter) {
            Some((_, previous)) => *previous = value,
            None => counts.push((counter, value)),
        }
    });
}

pub(crate) fn with_open(f: impl FnOnce(&mut Frame)) {
    with_store(|store| f(&mut store.open));
}

pub(crate) fn interact(cause: Cause) {
    with_store(|store| {
        let ts_us = store.now_us();
        if cause.rank() >= store.open.cause.rank() {
            store.open.cause = cause;
        }
        let name = cause.name();
        store.push(TraceEvent::Instant {
            name,
            cat: "input",
            ts_us,
        });
        if store.open_flow.is_none() {
            let id = store.next_flow;
            store.next_flow = store.next_flow.saturating_add(1);
            store.open_flow = Some((name, id, ts_us));
            store.push(TraceEvent::FlowStart { name, id, ts_us });
        }
    });
}

pub(crate) fn tick_frame() {
    if enabled() {
        let costs = costs();
        with_store(|store| close_frame(store, costs));
    }
}

/// The calibrated cost of one recorded span and one site run.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Costs {
    pub(crate) span_ns: u64,
    pub(crate) site_ns: u64,
}

pub(crate) fn costs() -> Costs {
    static COSTS: std::sync::OnceLock<Costs> = std::sync::OnceLock::new();
    *COSTS.get_or_init(calibrate)
}

fn calibrate() -> Costs {
    const RUNS: u32 = 4096;
    let saved = STORE.with(|cell| cell.replace(Store::new()));
    let per_run = |start: Instant| {
        u64::try_from(start.elapsed().as_nanos() / u128::from(RUNS)).unwrap_or(u64::MAX)
    };
    let start = Instant::now();
    for _ in 0..RUNS {
        drop(std::hint::black_box(crate::profile::span::Span::new(
            Phase::AppUpdate,
        )));
    }
    let span_ns = per_run(start);
    let location = core::panic::Location::caller();
    let start = Instant::now();
    for _ in 0..RUNS {
        drop(std::hint::black_box(crate::profile::site::site(
            location,
            SiteKind::Effect,
        )));
    }
    let site_ns = per_run(start);
    STORE.with(|cell| cell.replace(saved));
    Costs { span_ns, site_ns }
}

pub(crate) fn close_open() {
    if !enabled() {
        return;
    }
    let costs = costs();
    with_store(|store| {
        if !store.open.phases.is_empty() || !store.open.counts.is_empty() {
            close_frame(store, costs);
        }
    });
}

fn close_frame(store: &mut Store, costs: Costs) {
    let end_us = store.now_us();
    if let Some((name, id, _)) = store.open_flow.take() {
        store.push(TraceEvent::FlowFinish {
            name,
            id,
            ts_us: end_us,
        });
    }
    let mut frame = std::mem::take(&mut store.open);
    store.site_index.clear();
    store.attr_index.clear();
    store.attr_distinct.clear();
    frame
        .sites
        .sort_by_key(|site| std::cmp::Reverse(site.self_ns));
    for &(counter, value) in &frame.counts {
        store.push(TraceEvent::Counter {
            counter,
            ts_us: end_us,
            value,
        });
    }
    frame.zones = store.open_spans.saturating_add(store.open_sites);
    frame.overhead_ns = u64::from(store.open_spans)
        .saturating_mul(costs.span_ns)
        .saturating_add(u64::from(store.open_sites).saturating_mul(costs.site_ns));
    store.open_spans = 0;
    store.open_sites = 0;
    frame.index = store.next_index;
    frame.end_us = end_us;
    frame.wall_ns = store.open_start.take().map_or(0, |start| {
        u64::try_from(start.elapsed().as_nanos()).unwrap_or(0)
    });
    store.next_index = store.next_index.saturating_add(1);
    warn_slow(&frame);
    if store.frames.len() >= MAX_FRAMES {
        store.frames.pop_front();
    }
    store.frames.push_back(frame);
}

#[allow(clippy::cast_precision_loss, reason = "ns → ms display only")]
fn warn_slow(frame: &Frame) {
    if frame.work_ns() <= crate::profile::attr::budget_ns()
        || frame.cause == Cause::Idle
        || WARNS.load(Ordering::Relaxed) >= WARN_CAP
    {
        return;
    }
    WARNS.fetch_add(1, Ordering::Relaxed);
    let ms = frame.work_ns() as f64 / 1_000_000.0;
    let wait_ms = frame.total_ns().saturating_sub(frame.work_ns()) as f64 / 1_000_000.0;
    let (hot, peak) = frame
        .hottest()
        .map_or(("", 0), |(phase, ns)| (phase.name(), ns));
    let peak_ms = peak as f64 / 1_000_000.0;
    eprintln!(
        "sigil-profile: frame {} {} work={ms:.1}ms wait={wait_ms:.1}ms {hot}={peak_ms:.1} cmds={}",
        frame.index,
        frame.cause,
        frame.counter(Counter::SceneCmds)
    );
}

pub(crate) fn frames() -> Vec<Frame> {
    with_frames(|frames| frames.iter().cloned().collect())
}

pub(crate) fn with_frames<R>(f: impl FnOnce(&VecDeque<Frame>) -> R) -> R {
    STORE.with(|cell| f(&cell.borrow().frames))
}

pub(crate) fn events() -> Vec<TraceEvent> {
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
