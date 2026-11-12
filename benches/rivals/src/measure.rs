use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

pub struct Counting;

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

const WARM: usize = 50;
const UNITS: usize = 400;
const HEAVY_WARM: usize = 5;
const HEAVY_UNITS: usize = 40;
const WHEEL_RUN: usize = 20;
const SHOWN_WHEELS: usize = 5;

/// One change that a frame applies before it draws.
#[derive(Clone, Copy)]
pub enum Change {
    None,
    Grow,
    Point,
    Wheel(f32),
    Narrow,
    Wide,
}

impl Change {
    pub const fn narrow(self) -> Option<bool> {
        match self {
            Self::Narrow => Some(true),
            Self::Wide => Some(false),
            _ => None,
        }
    }
}

/// One library's chat scene, driven one frame at a time.
pub trait Rig {
    fn frame(&mut self, change: Change);

    fn show(&mut self) -> String;

    fn scoped(run: impl FnOnce()) {
        run();
    }
}

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

fn mount<R: Rig>(side: &str, make: &dyn Fn() -> R) {
    measure("mount", side, HEAVY_WARM, HEAVY_UNITS, |_| {
        R::scoped(|| drop(make()));
    });
}

fn first<R: Rig>(side: &str, make: &dyn Fn() -> R) {
    measure("first", side, HEAVY_WARM, HEAVY_UNITS, |_| {
        R::scoped(|| make().frame(Change::None));
    });
}

fn idle<R: Rig>(side: &str, make: &dyn Fn() -> R) {
    let mut rig = make();
    measure("idle", side, WARM, UNITS, |_| rig.frame(Change::None));
}

fn delta<R: Rig>(side: &str, make: &dyn Fn() -> R) {
    let mut rig = make();
    measure("delta", side, WARM, UNITS, |_| rig.frame(Change::Grow));
}

fn pointed<R: Rig>(make: &dyn Fn() -> R) -> R {
    let mut rig = make();
    rig.frame(Change::None);
    rig.frame(Change::Point);
    rig
}

fn scroll<R: Rig>(side: &str, make: &dyn Fn() -> R) {
    let mut rig = pointed(make);
    measure("scroll", side, WARM, UNITS, |unit| {
        let lines = if unit % (2 * WHEEL_RUN) < WHEEL_RUN {
            -1.0
        } else {
            1.0
        };
        rig.frame(Change::Wheel(lines));
    });
}

fn resize<R: Rig>(side: &str, make: &dyn Fn() -> R) {
    let mut rig = make();
    measure("resize", side, WARM, UNITS, |unit| {
        rig.frame(if unit % 2 == 0 {
            Change::Narrow
        } else {
            Change::Wide
        });
    });
}

fn show<R: Rig>(side: &str, make: &dyn Fn() -> R) {
    let mut rig = pointed(make);
    println!("SHOW {side} start\n{}", rig.show());
    for _ in 0..SHOWN_WHEELS {
        rig.frame(Change::Wheel(-1.0));
    }
    println!(
        "SHOW {side} after {SHOWN_WHEELS} wheel lines\n{}",
        rig.show()
    );
}

/// Runs the cells of one library, or the one cell that `only` names.
pub fn run<R: Rig>(side: &str, only: Option<&str>, make: &dyn Fn() -> R) {
    type Cell<R> = fn(&str, &dyn Fn() -> R);
    let cells: [(&str, Cell<R>, bool); 7] = [
        ("mount", mount, true),
        ("first", first, true),
        ("idle", idle, true),
        ("delta", delta, true),
        ("scroll", scroll, true),
        ("resize", resize, true),
        ("show", show, false),
    ];
    for (name, cell, timed) in cells {
        if only.map_or(timed, |wanted| wanted == name) {
            R::scoped(|| cell(side, make));
        }
    }
}
