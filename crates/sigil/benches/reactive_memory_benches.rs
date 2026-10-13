use std::cell::Cell;
use std::hint::black_box;
use std::mem;
use std::rc::Rc;
use std::time::{Duration, Instant};

use sigil::reactive::{Owner, Runtime, effect, signal};

use super::{bytes_allocated, fmt_delta, lcg_next, slot_shadow, with_alloc_delta};

pub fn report_slot_sizes() {
    let owner = mem::size_of::<slot_shadow::OwnerSlot>();
    let signal = mem::size_of::<slot_shadow::SignalSlot>();
    let effect = mem::size_of::<slot_shadow::EffectSlot>();
    println!("--- Slot inline sizes ---");
    println!("  OwnerSlot  = {owner} B  (parent + child index + 4 Vec headers + name + location)");
    println!(
        "  SignalSlot = {signal} B  (owner key + Box<dyn Any> + SmallVec<[EffectKey;4]> + location)"
    );
    println!(
        "  EffectSlot = {effect} B  (owner + run_owner + Box<dyn FnMut()> + 2×SmallVec + cursor + location + name)"
    );
    println!(
        "  (heap per signal additionally: sizeof(T) for the value box; subscribers spill past 4)"
    );
    println!("  (heap per effect additionally: sizeof(closure); each dep set spills past 4)");
    println!();
}

pub fn bench_signal_creation_bytes() -> i64 {
    const N: usize = 10_000;
    let mut bytes_per_signal = 0_i64;
    let mut total_delta = 0_i64;
    Runtime::scope("bench_mem_signal_create", || {
        let (_, delta) = with_alloc_delta(|| {
            let mut sigs = Vec::with_capacity(N);
            for _ in 0..N {
                sigs.push(signal(0_i32));
            }
            black_box(&sigs);
            mem::forget(sigs);
        });
        total_delta = delta;
        let holder_overhead = (N * mem::size_of::<sigil::reactive::Signal<i32>>()) as i64;
        bytes_per_signal = (delta - holder_overhead) / N as i64;
    });
    println!("--- Signal creation ---");
    println!(
        "  10_000 signals, total heap delta = {} ({} per signal incl. Box<i32>+slot+arena)",
        fmt_delta(total_delta),
        fmt_delta(bytes_per_signal),
    );
    let target: i64 = 128;
    let verdict = if bytes_per_signal <= target {
        "PASS"
    } else if bytes_per_signal <= target * 2 {
        "WARN"
    } else {
        "FAIL"
    };
    println!("  Budget = ≤ {target} B/signal → {verdict}");
    println!();
    bytes_per_signal
}

pub fn bench_effect_creation_bytes() {
    const N: usize = 10_000;
    let mut bytes_per_effect = 0_i64;
    let mut total_delta = 0_i64;
    Runtime::scope("bench_mem_effect_create", || {
        let n = signal(0_i32);
        let (_, delta) = with_alloc_delta(|| {
            let sinks: Vec<Rc<Cell<i32>>> = (0..N)
                .map(|_| {
                    let sink = Rc::new(Cell::new(0_i32));
                    let sink_for_effect = Rc::clone(&sink);
                    effect(move || sink_for_effect.set(n.get()));
                    sink
                })
                .collect();
            black_box(&sinks);
            mem::forget(sinks);
        });
        total_delta = delta;
        let holder_overhead = (N * mem::size_of::<Rc<Cell<i32>>>()) as i64;
        bytes_per_effect = (delta - holder_overhead) / N as i64;
    });
    println!("--- Effect creation ---");
    println!(
        "  10_000 effects, total delta = {} ({} per effect incl. closure+run_owner+slot)",
        fmt_delta(total_delta),
        fmt_delta(bytes_per_effect),
    );
    let target: i64 = 512;
    let verdict = if bytes_per_effect <= target {
        "PASS"
    } else {
        "WARN"
    };
    println!("  Reference target = ≤ {target} B/effect → {verdict}");
    println!();
}

pub fn bench_create_dispose_cycles_no_growth(steady_bytes_per_signal: i64) {
    const CYCLES: usize = 10;
    const PER_CYCLE: usize = 10_000;
    let baseline = bytes_allocated() as i64;
    let mut deltas: Vec<i64> = Vec::with_capacity(CYCLES);
    for _ in 0..CYCLES {
        let cycle_start = bytes_allocated() as i64;
        Runtime::scope("bench_mem_dispose_cycle", || {
            let mut sigs = Vec::with_capacity(PER_CYCLE);
            for _ in 0..PER_CYCLE {
                sigs.push(signal(0_i32));
            }
            black_box(&sigs);
        });
        let cycle_end = bytes_allocated() as i64;
        deltas.push(cycle_end - cycle_start);
    }
    let final_delta = bytes_allocated() as i64 - baseline;
    let budget = steady_bytes_per_signal * PER_CYCLE as i64 * 2;
    println!("--- Leak check: 10 × (create 10_000 signals → dispose) ---");
    for (i, d) in deltas.iter().enumerate() {
        println!("  cycle {:>2}: per-cycle delta = {}", i + 1, fmt_delta(*d));
    }
    println!(
        "  net residual after 10 cycles = {} (budget ≤ 2× single-batch = {})",
        fmt_delta(final_delta),
        fmt_delta(budget),
    );
    let verdict = if final_delta.abs() <= budget {
        "PASS"
    } else {
        "FAIL"
    };
    println!("  Verdict: {verdict}");
    println!();
}

pub fn bench_streaming_session_10_seconds() {
    const NUM_SIGNALS: usize = 100;
    const SECONDS: u64 = 10;
    let mut writes_done = 0_u64;
    let mut peak_delta = 0_i64;
    Runtime::scope("bench_mem_streaming", || {
        let signals: Vec<_> = (0..NUM_SIGNALS).map(|_| signal(0_i32)).collect();
        let sinks: Vec<Rc<Cell<i32>>> = signals
            .iter()
            .map(|&s| {
                let sink = Rc::new(Cell::new(0_i32));
                let sink_for_effect = Rc::clone(&sink);
                effect(move || sink_for_effect.set(s.get()));
                sink
            })
            .collect();
        let baseline = bytes_allocated() as i64;
        let deadline = Instant::now() + Duration::from_secs(SECONDS);
        let mut rng: u64 = 0x5dee_f00d;
        let mut counter = 0_i32;
        while Instant::now() < deadline {
            for _ in 0..1000 {
                let idx = (lcg_next(&mut rng) as usize) % NUM_SIGNALS;
                counter = counter.wrapping_add(1);
                signals[idx].set(black_box(counter));
                writes_done += 1;
            }
            let delta = bytes_allocated() as i64 - baseline;
            if delta > peak_delta {
                peak_delta = delta;
            }
        }
        let _ = (signals, sinks);
    });
    println!("--- Streaming session (10s, 100 signals × 100 effects) ---");
    println!("  writes performed = {writes_done}");
    println!("  peak heap delta during run = {}", fmt_delta(peak_delta));
    let budget_bytes: i64 = 10 * 1024 * 1024;
    let verdict = if peak_delta <= budget_bytes {
        "PASS"
    } else {
        "FAIL"
    };
    println!("  Budget = ≤ 10 MB (10-minute target, scaled to the run) → {verdict}");
    let projected_10min = (peak_delta as f64) * (60.0 * 10.0 / SECONDS as f64);
    println!(
        "  Naive 10-min projection (linear) = {}",
        fmt_delta(projected_10min as i64)
    );
    println!();
}

pub fn bench_deep_tree_disposal_1000_nodes() {
    const DEPTH: usize = 1000;
    let mut elapsed = Duration::ZERO;
    let mut residual_after_dispose = 0_i64;
    Runtime::scope("bench_mem_deep_tree", || {
        let baseline = bytes_allocated() as i64;
        let root = Owner::new("root", Owner::current());
        let mut current_id = root.id();
        let mut leaves: Vec<Owner> = Vec::with_capacity(DEPTH);
        leaves.push(root);
        for _ in 0..DEPTH - 1 {
            let child = Owner::new("node", Some(current_id));
            current_id = child.id();
            leaves.push(child);
        }
        let root_owner = *leaves.first().expect("root present");
        drop(leaves);
        let start = Instant::now();
        root_owner.dispose();
        elapsed = start.elapsed();
        residual_after_dispose = bytes_allocated() as i64 - baseline;
    });
    println!("--- Deep tree disposal (1000 nested owners) ---");
    println!("  disposal time = {elapsed:?}");
    println!(
        "  residual heap after dispose = {}",
        fmt_delta(residual_after_dispose)
    );
    let time_verdict = if elapsed <= Duration::from_micros(100) {
        "PASS"
    } else if elapsed <= Duration::from_micros(500) {
        "WARN"
    } else {
        "FAIL"
    };
    let heap_verdict = if residual_after_dispose.unsigned_abs() <= 10 * 1024 {
        "PASS"
    } else {
        "WARN"
    };
    println!("  Time budget = ≤ 100 µs → {time_verdict}");
    println!("  Heap budget = ≤ 10 KB residual → {heap_verdict}");
    println!();
}
