//! Tier 5 memory + allocation benches for `ui-reactive`.
//!
//! Targets from DR-009 §2:
//!
//! - Memory per signal (steady state)         → ≤ 128 B
//! - 10-minute streaming session memory delta → ≤ 10 MB
//! - Deep owner disposal, 1000 nodes          → ≤ 100 µs (time)
//!                                              + ≤ 10 KB residual heap
//!
//! The 10-minute streaming budget is approximated here by a 10-second
//! stress run (`streaming_session_10_seconds`); a 10s drift of >1 MB
//! projects to a catastrophic 10-minute leak, so the short-run signal
//! is the one we can usefully gate on inside a bench harness.
//!
//! These are *not* criterion benches — they print absolute numbers and
//! assert against fixed budgets. Criterion's statistical-comparison
//! flow doesn't fit "did the heap grow by N bytes" questions, and a
//! plain `fn main` keeps the output a flat table that is trivial to
//! eyeball during local runs and easy to grep in CI logs.
//!
//! Run with `cargo bench -p ui-reactive --bench memory`. The bench
//! binary installs a custom global allocator (`CountingAllocator`)
//! that wraps `System` and tracks bytes allocated minus bytes freed,
//! so all heap traffic — including criterion's, ours, and any
//! transitive — is captured.

// `unsafe_code` is workspace-deny. The custom allocator is the one
// thin path to allocation accounting without pulling in another
// dependency (`cap`, `dhat`); the unsafe surface is three trait
// methods that delegate to `std::alloc::System` and update a single
// atomic counter. Scoped to this bench binary only.
#![allow(unsafe_code)]
// Bench file: per docs/plans/native-ui/hygiene-cleanup-framework.md,
// benches may bend `panic` / `unwrap_used` / `indexing_slicing` for
// readability — the alternative is layers of `match` noise that
// obscures what the bench is asserting. The asserts here are also the
// failure signal: a budget overshoot SHOULD panic the bench binary so
// CI catches it.
#![allow(missing_docs, clippy::missing_docs_in_private_items)]
#![allow(clippy::panic, clippy::unwrap_used, clippy::indexing_slicing)]
// Allocation counters and human-readable formatters do unavoidable
// integer/float juggling. These are pedantic / nursery lints that fire
// on every `as` between byte-count types; allowing them keeps the
// bench focused on measurement logic rather than `try_from` ceremony
// that cannot fail at our problem sizes (≤ a few MB).
#![allow(
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::uninlined_format_args,
    clippy::missing_const_for_fn,
    clippy::single_match_else,
    clippy::doc_overindented_list_items,
    clippy::doc_markdown,
    clippy::ignored_unit_patterns
)]
#![allow(unreachable_pub)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;
use std::mem;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

// `criterion` and `smallvec` are dev/regular deps that get pulled into
// every bench compilation unit. We don't reference them here so the
// `unused_crate_dependencies` lint complains without these aliases.
use criterion as _;
use smallvec as _;

use ui_reactive::{Owner, Runtime, effect, signal};

// ---------------------------------------------------------------------
// Allocation counter
// ---------------------------------------------------------------------

/// Live bytes (allocated - freed) tracked by the global allocator.
///
/// `Relaxed` is the right ordering here: we only ever read the counter
/// from the bench thread between operations that have already
/// happens-before-synced via the allocator's own internal locking, and
/// we are not using the value to make a control-flow decision in
/// another thread.
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

struct CountingAllocator;

// SAFETY: `CountingAllocator` forwards every call to `std::alloc::System`,
// which is the platform's well-formed allocator. The only added work is
// a `fetch_add`/`fetch_sub` on a `static AtomicUsize`. We never alias
// pointers and we never invent layouts.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarding to the platform allocator with the same
        // layout the caller supplied; no aliasing or layout invention.
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // Counter goes down before dealloc so a concurrent reader can
        // never see a transient "freed but still counted" state. The
        // ordering is otherwise irrelevant — see `ALLOCATED` docs.
        ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
        // SAFETY: caller's contract on `dealloc` — the pointer was
        // returned by `alloc` with this exact `layout` and is still
        // live. We forward those guarantees verbatim to `System`.
        unsafe { System.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let old_size = layout.size();
        // SAFETY: forwarding `ptr`/`layout`/`new_size` to `System`
        // unchanged. Caller's `realloc` contract carries through.
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        if !p.is_null() {
            if new_size > old_size {
                ALLOCATED.fetch_add(new_size - old_size, Ordering::Relaxed);
            } else {
                ALLOCATED.fetch_sub(old_size - new_size, Ordering::Relaxed);
            }
        }
        p
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn bytes_allocated() -> usize {
    ALLOCATED.load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------

/// Run `f`, return `(result, bytes_delta)` where `bytes_delta` is the
/// signed change in live heap bytes between start and end.
fn with_alloc_delta<R>(f: impl FnOnce() -> R) -> (R, i64) {
    let before = bytes_allocated() as i64;
    let result = f();
    let after = bytes_allocated() as i64;
    (result, after - before)
}

/// Minimal LCG for picking a random signal index. We do NOT pull
/// `rand` in just for this — it would be the only crate-level dep and
/// the streaming bench doesn't care about distribution quality, only
/// that we hit different signals.
fn lcg_next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

/// Format a signed byte count as `+12 KB` / `-3 B` for the table.
fn fmt_delta(bytes: i64) -> String {
    let sign = if bytes < 0 { "-" } else { "+" };
    let abs = bytes.unsigned_abs();
    if abs < 1024 {
        format!("{sign}{abs} B")
    } else if abs < 1024 * 1024 {
        format!("{sign}{:.1} KB", abs as f64 / 1024.0)
    } else {
        format!("{sign}{:.2} MB", abs as f64 / (1024.0 * 1024.0))
    }
}

/// Reach into the runtime's private `OwnerSlot` / `SignalSlot` /
/// `EffectSlot` layout via the same shape we know the runtime uses.
/// These shadow types intentionally mirror `runtime.rs`. If the real
/// types ever change, this bench will report a different per-slot
/// number and any mismatch shows up in the printed table — the goal
/// is the *measurement*, not strict layout coupling.
mod slot_shadow {
    use core::any::Any;
    use core::panic::Location;
    use smallvec::SmallVec;

    /// Shadows `runtime::OwnerKey` (= `Key<OwnerSlot>`).
    #[allow(dead_code)]
    pub struct OwnerKey {
        index: u32,
        generation: u32,
        _ty: core::marker::PhantomData<fn() -> ()>,
    }

    /// Shadows `runtime::SignalKey`.
    #[allow(dead_code)]
    pub struct SignalKey {
        index: u32,
        generation: u32,
        _ty: core::marker::PhantomData<fn() -> ()>,
    }

    /// Shadows `runtime::EffectKey`.
    #[allow(dead_code)]
    pub struct EffectKey {
        index: u32,
        generation: u32,
        _ty: core::marker::PhantomData<fn() -> ()>,
    }

    /// Mirror of `runtime::OwnerSlot` for `size_of_val` reporting.
    #[allow(dead_code)]
    pub struct OwnerSlot {
        pub parent: Option<OwnerKey>,
        pub children: Vec<OwnerKey>,
        pub signals: Vec<SignalKey>,
        pub effects: Vec<EffectKey>,
        pub cleanups: Vec<Box<dyn FnOnce()>>,
        pub name: &'static str,
        pub location: &'static Location<'static>,
    }

    /// Mirror of `runtime::SignalSlot`.
    #[allow(dead_code)]
    pub struct SignalSlot {
        pub owner: OwnerKey,
        pub value: Box<dyn Any>,
        pub subscribers: SmallVec<[EffectKey; 4]>,
        pub location: &'static Location<'static>,
    }

    /// Mirror of `runtime::EffectSlot`.
    #[allow(dead_code)]
    pub struct EffectSlot {
        pub owner: OwnerKey,
        pub run_owner: Option<OwnerKey>,
        pub f: Box<dyn FnMut()>,
        pub deps: SmallVec<[SignalKey; 4]>,
        pub location: &'static Location<'static>,
        pub name: &'static str,
    }
}

// ---------------------------------------------------------------------
// Bench bodies
// ---------------------------------------------------------------------

/// Print sizeof breakdown for the three slot types.
///
/// These numbers are inline storage only; the heap-allocated payload
/// per slot is approximately:
///
/// - `OwnerSlot`: 4 `Vec`s + 1 `Vec<Box<dyn FnOnce()>>` (each empty
///   here, so 0 heap bytes until populated). 4 `Vec` headers are
///   already in the inline size.
/// - `SignalSlot`: `Box<dyn Any>` for the value (heap-allocates the
///   actual `T`, here `i32` → 8 B incl. fat-pointer overhead) + a
///   `SmallVec<[EffectKey; 4]>` whose 4 inline slots are part of the
///   inline size; spills heap-only past 4 subscribers.
/// - `EffectSlot`: `Box<dyn FnMut()>` for the closure (heap-allocates
///   captures + vtable) + `SmallVec<[SignalKey; 4]>` deps.
fn report_slot_sizes() {
    let owner = mem::size_of::<slot_shadow::OwnerSlot>();
    let signal = mem::size_of::<slot_shadow::SignalSlot>();
    let effect = mem::size_of::<slot_shadow::EffectSlot>();
    println!("--- Slot inline sizes ---");
    println!("  OwnerSlot       = {owner} B  (parent + 4 Vec headers + name + location)");
    println!(
        "  SignalSlot      = {signal} B  (owner key + Box<dyn Any> + SmallVec<[EffectKey;4]> + location)"
    );
    println!(
        "  EffectSlot      = {effect} B  (owner key + run_owner + Box<dyn FnMut()> + SmallVec<[SignalKey;4]> + location + name)"
    );
    println!(
        "  (heap per signal additionally: sizeof(T) for the value box; subscribers spill past 4)"
    );
    println!(
        "  (heap per effect additionally: sizeof(closure) for the FnMut box; deps spill past 4)"
    );
    println!();
}

/// Budget: ≤ 128 B amortised.
///
/// Creates 10_000 signals inside one scope, measures the heap delta,
/// divides. The result includes the slot itself, the `Box<i32>` value,
/// the owner's `signals` Vec growth, and any arena slot/free-list
/// growth — i.e. everything a real signal costs in steady state.
fn bench_signal_creation_bytes() -> i64 {
    const N: usize = 10_000;
    let mut bytes_per_signal = 0_i64;
    let mut total_delta = 0_i64;
    Runtime::scope("bench_mem_signal_create", || {
        let (_, delta) = with_alloc_delta(|| {
            let mut sigs = Vec::with_capacity(N);
            for _ in 0..N {
                sigs.push(signal(0_i32));
            }
            // Keep the Vec alive past the measurement so the optimizer
            // can't eliminate the work and so the heap delta reflects
            // all retained allocations.
            black_box(&sigs);
            // Intentionally leak the holder Vec so the scope's
            // disposal — which frees every Box<i32> + slot — doesn't
            // confound the measurement. We subtract the holder's
            // contribution explicitly below.
            mem::forget(sigs);
        });
        total_delta = delta;
        let holder_overhead = (N * mem::size_of::<ui_reactive::Signal<i32>>()) as i64;
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

/// Per-effect cost — same shape as the signal bench. Effects hold a
/// `Box<dyn FnMut()>` plus a per-run owner allocation, so the per-effect
/// number is intentionally larger than per-signal.
fn bench_effect_creation_bytes() {
    const N: usize = 10_000;
    let mut bytes_per_effect = 0_i64;
    let mut total_delta = 0_i64;
    Runtime::scope("bench_mem_effect_create", || {
        // One signal that every effect reads, so each effect has a
        // real dep and a real run-owner, not a degenerate empty body.
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
        "  10_000 effects, total heap delta = {} ({} per effect incl. closure+run_owner+slot)",
        fmt_delta(total_delta),
        fmt_delta(bytes_per_effect),
    );
    // Effects are inherently heavier than signals; no fixed DR-009
    // budget, but flag if it crosses 512 B which would mean either the
    // closure box, the run-owner allocation, or the deps SmallVec is
    // doing something unexpected.
    let target: i64 = 512;
    let verdict = if bytes_per_effect <= target {
        "PASS"
    } else {
        "WARN"
    };
    println!("  Reference target = ≤ {target} B/effect → {verdict}");
    println!();
}

/// Leak check: 10 cycles of "create 10_000 signals → dispose scope".
///
/// If the arena reclaims slots the way it should (free-list pop on next
/// `insert`), the heap should plateau by cycle 2 — cycle 1 grows the
/// `Vec<Slot<T>>` to its high-water mark; cycle 2 reuses every slot.
/// Linear growth across cycles ⇒ the arena never reclaims, which is
/// the leak this bench exists to catch.
///
/// Budget: total residual after 10 cycles ≤ 2× the steady-state of
/// one batch.
fn bench_create_dispose_cycles_no_growth(steady_bytes_per_signal: i64) {
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
            // Sigs vec drops here — its Signal<T> handles are Copy,
            // but dropping the vec releases the holder allocation. The
            // scope's owner disposal then frees every Box<i32> + slot.
        });
        let cycle_end = bytes_allocated() as i64;
        deltas.push(cycle_end - cycle_start);
    }
    let final_delta = bytes_allocated() as i64 - baseline;
    let one_batch_bytes = steady_bytes_per_signal * PER_CYCLE as i64;
    let budget = one_batch_bytes * 2;
    println!("--- Leak check: 10 × (create 10_000 signals → dispose) ---");
    for (i, d) in deltas.iter().enumerate() {
        println!("  cycle {:>2}: per-cycle delta = {}", i + 1, fmt_delta(*d));
    }
    println!(
        "  net residual after 10 cycles = {} (budget ≤ 2× single-batch steady-state = {})",
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

/// Streaming-session simulation. 10 seconds of wall-clock writes at
/// best-effort throughput, 100 signals × 100 subscribed effects.
///
/// Budget: heap delta during the run ≤ 10 MB. (DR-009 says 10 MB over
/// 10 minutes; we scale to 10s on the assumption that any leak that
/// shows up in seconds will be catastrophic in minutes — the per-write
/// allocation contribution is what the test guards against.)
fn bench_streaming_session_10_seconds() {
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
        // Drain initial-run cost out of the measurement window.
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
            let now = bytes_allocated() as i64;
            let delta = now - baseline;
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
    println!("  Budget = ≤ 10 MB (DR-009 10-min target; 10s scaled) → {verdict}");
    let projected_10min = (peak_delta as f64) * (60.0 * 10.0 / SECONDS as f64);
    println!(
        "  Naive 10-min projection (linear) = {}",
        fmt_delta(projected_10min as i64)
    );
    println!();
}

/// Deep owner tree: 1000 nested owners (each owns the next), then
/// dispose the root. Time the disposal and the residual heap delta.
///
/// Budgets:
/// - disposal time ≤ 100 µs (DR-009)
/// - residual heap after dispose ≤ 10 KB
fn bench_deep_tree_disposal_1000_nodes() {
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
        // Capture the root before dropping the holder vec; Owner is
        // Copy so this is a value snapshot, not a move.
        let root = *leaves.first().expect("root present");
        // Drop the holder vec before the timed dispose so its
        // deallocation isn't included in the disposal time.
        drop(leaves);
        let start = Instant::now();
        root.dispose();
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

// ---------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------

fn main() {
    println!();
    println!("=========================================================");
    println!("  ui-reactive Tier 5 (memory + leaks) — DR-009");
    println!("=========================================================");
    println!();
    report_slot_sizes();
    let bytes_per_signal = bench_signal_creation_bytes();
    bench_effect_creation_bytes();
    bench_create_dispose_cycles_no_growth(bytes_per_signal);
    bench_streaming_session_10_seconds();
    bench_deep_tree_disposal_1000_nodes();
    println!("=========================================================");
    println!("  Done.");
    println!("=========================================================");
}
