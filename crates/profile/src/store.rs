//! Thread-local event log.

use std::cell::RefCell;
use std::env;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::Instant;

use crate::event::Event;

const MAX_EVENTS: usize = 8_192;
const MAX_FRAMES: usize = 240;
const ENV_UNSET: u8 = 0;
const ENV_OFF: u8 = 1;
const ENV_ON: u8 = 2;
const WARN_CAP: u8 = 8;

/// 60 Hz frame budget in nanoseconds.
pub const FRAME_BUDGET_NS: u64 = 16_670_000;
/// Chrome INP threshold (200 ms). Not the native click bar.
/// Native interaction uses [`FRAME_BUDGET_NS`].
pub const INTERACT_BUDGET_NS: u64 = 200_000_000;

static FORCED: AtomicBool = AtomicBool::new(false);
static TEST_OFF: AtomicBool = AtomicBool::new(false);
static ENV: AtomicU8 = AtomicU8::new(ENV_UNSET);
static WARNS: AtomicU8 = AtomicU8::new(0);

thread_local! {
    static STORE: RefCell<Store> = RefCell::new(Store::new());
}

/// One named duration or counter on a frame.
pub type Phase = (&'static str, u64);

/// One closed frame.
#[derive(Clone, Debug)]
pub struct Frame {
    /// 0-based index.
    pub index: u32,
    /// Input or scheduler that caused this frame.
    pub cause: &'static str,
    /// Wall-clock nanoseconds from [`begin_frame`](super::begin_frame) to tick.
    pub wall_ns: u64,
    /// Phase name + nanoseconds (nested phases included).
    pub phases: Vec<Phase>,
    /// Counters for this frame.
    pub counts: Vec<Phase>,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            index: 0,
            cause: "idle",
            wall_ns: 0,
            phases: Vec::new(),
            counts: Vec::new(),
        }
    }
}

impl Frame {
    /// Counter value, or 0.
    #[must_use]
    pub fn counter(&self, name: &str) -> u64 {
        self.counts
            .iter()
            .find(|(n, _)| *n == name)
            .map_or(0, |(_, v)| *v)
    }

    /// Wall-clock frame time, or the sum of top-level phases if unset.
    #[must_use]
    pub fn total_ns(&self) -> u64 {
        if self.wall_ns > 0 {
            return self.wall_ns;
        }
        self.phases
            .iter()
            .filter(|(n, _)| !n.contains('.'))
            .map(|(_, ns)| *ns)
            .sum()
    }

    /// Phase with the largest duration.
    #[must_use]
    pub fn hottest(&self) -> Option<Phase> {
        self.phases.iter().copied().max_by_key(|(_, ns)| *ns)
    }
}

struct Store {
    origin: Instant,
    events: Vec<Event>,
    frames: Vec<Frame>,
    open: Frame,
    next_index: u32,
    open_start: Option<Instant>,
    next_flow: u64,
    open_flow: Option<(&'static str, u64, u64)>,
    cause_rank: u8,
}

impl Store {
    fn new() -> Self {
        Self {
            origin: Instant::now(),
            events: Vec::with_capacity(1024),
            frames: Vec::with_capacity(64),
            open: Frame::default(),
            next_index: 0,
            open_start: None,
            next_flow: 1,
            open_flow: None,
            cause_rank: 0,
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

pub(crate) fn begin_frame() {
    if !enabled() {
        return;
    }
    STORE.with(|cell| {
        let mut store = cell.borrow_mut();
        store.open_start = Some(Instant::now());
        store.cause_rank = 0;
        store.open.cause = "idle";
    });
}

pub(crate) fn complete(name: &'static str, cat: &'static str, start_us: u64, dur_ns: u64) {
    if !enabled() {
        return;
    }
    STORE.with(|cell| {
        let mut store = cell.borrow_mut();
        let micros = (dur_ns / 1_000).max(1);
        push_event(
            &mut store,
            Event::Complete {
                name,
                cat,
                tid: if cat == "gpu" { 2 } else { 1 },
                start_us,
                dur_us: micros,
            },
        );
        push_phase(&mut store.open.phases, name, dur_ns);
    });
}

pub(crate) fn count_add(name: &'static str, n: u64) {
    if !enabled() || n == 0 {
        return;
    }
    STORE.with(|cell| {
        push_phase(&mut cell.borrow_mut().open.counts, name, n);
    });
}

pub(crate) fn interact(name: &'static str) {
    if !enabled() {
        return;
    }
    STORE.with(|cell| {
        let mut store = cell.borrow_mut();
        let ts = store.now_us();
        let rank = cause_rank(name);
        if rank >= store.cause_rank {
            store.cause_rank = rank;
            store.open.cause = name;
        }
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
    let counts = store.open.counts.clone();
    for &(name, value) in &counts {
        push_event(
            store,
            Event::Counter {
                name,
                ts_us: end_us,
                value,
            },
        );
    }
    let mut frame = std::mem::take(&mut store.open);
    frame.index = store.next_index;
    frame.wall_ns = store.open_start.take().map_or(0, |start| {
        u64::try_from(start.elapsed().as_nanos()).unwrap_or(0)
    });
    store.next_index = store.next_index.saturating_add(1);
    if warn {
        warn_slow(&frame);
    }
    if store.frames.len() >= MAX_FRAMES {
        store.frames.remove(0);
    }
    store.frames.push(frame);
    store.cause_rank = 0;
}

fn warn_slow(frame: &Frame) {
    if frame.total_ns() <= FRAME_BUDGET_NS || frame.cause == "idle" {
        return;
    }
    if WARNS.load(Ordering::Relaxed) >= WARN_CAP {
        return;
    }
    WARNS.fetch_add(1, Ordering::Relaxed);
    #[allow(clippy::cast_precision_loss, reason = "ns → ms display only")]
    let ms = frame.total_ns() as f64 / 1_000_000.0;
    let (hot, peak) = frame.hottest().unwrap_or(("", 0));
    #[allow(clippy::cast_precision_loss, reason = "ns → ms display only")]
    let peak_ms = peak as f64 / 1_000_000.0;
    eprintln!(
        "sigil-profile: frame {} {} {ms:.1}ms {hot}={peak_ms:.1} cmds={}",
        frame.index,
        frame.cause,
        frame.counter("scene.cmds")
    );
}

fn cause_rank(name: &str) -> u8 {
    match name {
        "pointer" | "key" => 5,
        "scroll" | "ime" => 4,
        "hover" | "resize" | "a11y" => 3,
        "animation" | "interval" => 2,
        _ => 1,
    }
}

fn push_event(store: &mut Store, event: Event) {
    if store.events.len() >= MAX_EVENTS {
        store.events.remove(0);
    }
    store.events.push(event);
}

fn push_phase(list: &mut Vec<Phase>, name: &'static str, add: u64) {
    if let Some((_, ns)) = list.iter_mut().find(|(n, _)| *n == name) {
        *ns = ns.saturating_add(add);
    } else {
        list.push((name, add));
    }
}

pub(crate) fn frames() -> Vec<Frame> {
    STORE.with(|cell| cell.borrow().frames.clone())
}

pub(crate) fn events() -> Vec<Event> {
    STORE.with(|cell| cell.borrow().events.clone())
}
