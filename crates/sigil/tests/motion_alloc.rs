#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    unsafe_code,
    reason = "a counting global allocator must implement the unsafe GlobalAlloc trait"
)]
#![allow(clippy::unwrap_used)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use sigil::animation::{Keyframes, Property};
use sigil::prelude::*;
use sigil::reactive::Owner;

const CALLS: u32 = 1_000;
const BUDGET: i64 = 4_096;

thread_local! {
    static NET: Cell<i64> = const { Cell::new(0) };
}

static SERIAL: Mutex<()> = Mutex::new(());

fn count(bytes: usize, sign: i64) {
    let bytes = i64::try_from(bytes).unwrap_or(i64::MAX);
    let _ = NET.try_with(|net| net.set(net.get() + sign * bytes));
}

struct Counting;

// SAFETY: every call forwards to the system allocator with the same arguments.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count(layout.size(), 1);
        // SAFETY: the caller upholds the GlobalAlloc contract for `layout`.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        count(layout.size(), -1);
        // SAFETY: `ptr` came from `alloc` with this `layout`.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count(new_size, 1);
        count(layout.size(), -1);
        // SAFETY: the caller upholds the GlobalAlloc realloc contract.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn fade(opacity: f32) -> Keyframes {
    Keyframes::new(Duration::from_millis(300))
        .keyframe(0.0, Property::Opacity(opacity))
        .keyframe(1.0, Property::TranslateX(4.0))
}

fn animate(keyframes: Keyframes) {
    drop(container().animate(keyframes));
}

fn retained(warm: u32, mut keyframes: impl FnMut(u32) -> Keyframes) -> i64 {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    for call in 0..warm {
        animate(keyframes(call));
    }
    let before = NET.with(Cell::get);
    for call in warm..warm + CALLS {
        animate(keyframes(call));
    }
    NET.with(Cell::get) - before
}

#[test]
fn equal_finite_keyframes_share_one_entry() {
    let bytes = retained(1, |_| fade(0.5));
    assert!(bytes < BUDGET, "1000 equal keyframes kept {bytes} bytes");
}

#[test]
fn equal_nan_keyframes_share_one_entry() {
    let bytes = retained(1, |_| fade(f32::NAN));
    assert!(bytes < BUDGET, "1000 equal NaN keyframes kept {bytes} bytes");
}

#[test]
fn keyframes_built_from_changing_values_stay_bounded() {
    let bytes = retained(CALLS, |call| fade(call as f32 / 4_096.0));
    assert!(bytes < BUDGET, "1000 distinct keyframes kept {bytes} bytes after warm-up");
}

#[test]
fn keyframes_pinned_by_disposed_scopes_are_released() {
    let bytes = retained(CALLS, |call| {
        let keyframes = fade(0.25 + call as f32 / 8_192.0);
        Owner::scope("pinned-animation", || drop(container().animate(keyframes.clone())));
        keyframes
    });
    assert!(bytes < BUDGET, "1000 scoped distinct keyframes kept {bytes} bytes after warm-up");
}
