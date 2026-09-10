#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(unsafe_code)]
#![allow(missing_docs, clippy::missing_docs_in_private_items)]
#![allow(clippy::panic, clippy::unwrap_used, clippy::indexing_slicing)]
#![allow(clippy::print_stdout)]
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

mod memory_benches;

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

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

pub(crate) fn bytes_allocated() -> usize {
    ALLOCATED.load(Ordering::Relaxed)
}

pub(crate) fn with_alloc_delta<R>(f: impl FnOnce() -> R) -> (R, i64) {
    let before = bytes_allocated() as i64;
    let result = f();
    let after = bytes_allocated() as i64;
    (result, after - before)
}

pub(crate) fn lcg_next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

pub(crate) fn fmt_delta(bytes: i64) -> String {
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

pub(crate) mod slot_shadow {
    use core::any::Any;
    use core::panic::Location;
    use smallvec::SmallVec;

    #[allow(dead_code, reason = "bench harness; deliberate")]
    pub struct OwnerKey {
        index: u32,
        generation: u32,
        _ty: core::marker::PhantomData<fn() -> ()>,
    }

    #[allow(dead_code, reason = "bench harness; deliberate")]
    pub struct SignalKey {
        index: u32,
        generation: u32,
        _ty: core::marker::PhantomData<fn() -> ()>,
    }

    #[allow(dead_code, reason = "bench harness; deliberate")]
    pub struct EffectKey {
        index: u32,
        generation: u32,
        _ty: core::marker::PhantomData<fn() -> ()>,
    }

    #[allow(dead_code, reason = "bench harness; deliberate")]
    pub struct OwnerSlot {
        pub parent: Option<OwnerKey>,
        pub index_in_parent: Option<usize>,
        pub children: Vec<OwnerKey>,
        pub signals: Vec<SignalKey>,
        pub effects: Vec<EffectKey>,
        pub cleanups: Vec<Box<dyn FnOnce()>>,
        pub name: &'static str,
        pub location: &'static Location<'static>,
    }

    #[allow(dead_code, reason = "bench harness; deliberate")]
    pub struct SignalSlot {
        pub owner: OwnerKey,
        pub value: Box<dyn Any>,
        pub subscribers: SmallVec<[EffectKey; 4]>,
        pub location: &'static Location<'static>,
    }

    #[allow(dead_code, reason = "bench harness; deliberate")]
    pub struct EffectSlot {
        pub owner: OwnerKey,
        pub run_owner: Option<OwnerKey>,
        pub f: Box<dyn FnMut()>,
        pub deps: SmallVec<[SignalKey; 4]>,
        pub prev_deps: SmallVec<[SignalKey; 4]>,
        pub prev_dep_cursor: usize,
        pub location: &'static Location<'static>,
        pub name: &'static str,
    }
}

fn main() {
    println!();
    println!("=========================================================");
    println!("  sigil-reactive Tier 5 (memory + leaks) — DR-009");
    println!("=========================================================");
    println!();
    memory_benches::report_slot_sizes();
    let bytes_per_signal = memory_benches::bench_signal_creation_bytes();
    memory_benches::bench_effect_creation_bytes();
    memory_benches::bench_create_dispose_cycles_no_growth(bytes_per_signal);
    memory_benches::bench_streaming_session_10_seconds();
    memory_benches::bench_deep_tree_disposal_1000_nodes();
    println!("=========================================================");
    println!("  Done.");
    println!("=========================================================");
}
